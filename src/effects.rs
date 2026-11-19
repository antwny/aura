//! Lifecycle manager for Aura's independent Wayland effects renderer.

use crate::config::Config;
use crate::engine::outputs::MonitorOutput;
use serde::Serialize;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectsStatus {
    Disabled,
    Running,
    Failed(String),
}

#[derive(Default)]
pub struct EffectsManager {
    child: Option<Child>,
    config_path: Option<PathBuf>,
    last_config: Option<String>,
    last_error: Option<String>,
    desired_paused: Vec<u8>,
    last_sent_paused: Option<Vec<u8>>,
    control_stdin: Option<ChildStdin>,
    control_pipe_buf: usize,
    control_error: Option<String>,
    palette_cache: std::collections::HashMap<(PathBuf, bool), (u64, ([u8; 3], [u8; 3], [u8; 3]))>,
}

#[derive(Serialize)]
struct EffectsConfig {
    outputs: Vec<EffectOutput>,
}

#[derive(Serialize)]
struct EffectOutput {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parallax: Option<Parallax>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spectrum: Option<Spectrum>,
}

#[derive(Serialize)]
struct Parallax {
    image: String,
    depth: String,
    strength: f32,
}

#[derive(Serialize)]
struct Spectrum {
    bars: u32,
    height_pct: u32,
    color1: [u8; 3],
    color2: [u8; 3],
    halo: [u8; 3],
    alpha: f32,
    position: &'static str,
    style: &'static str,
    bar_width_pct: u32,
    glow: bool,
}

/// Effects are drawn as wlr-layer-shell surfaces. GNOME does not implement
/// that protocol on Wayland, and X11 sessions have no Wayland connection at
/// all, so spawning the renderer there only produces a process that exits
/// immediately. `AURA_EFFECTS_FORCE=1` allows experimenting anyway.
pub fn renderer_supported() -> bool {
    if std::env::var_os("AURA_EFFECTS_FORCE").is_some_and(|value| value != "0") {
        return true;
    }
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return false;
    }
    !matches!(
        crate::engine::DesktopEnvironment::detect(),
        crate::engine::DesktopEnvironment::Gnome
    )
}

/// Niri stacks layer-shell surfaces of the same layer by creation order, and
/// mpvpaper recreates its surface on every apply. Only there must the effects
/// surfaces be recreated afterwards. Everywhere else the renderer stays alive
/// so cava and the GPU device are not restarted at wallpaper-apply time.
pub fn restart_after_wallpaper_apply() -> bool {
    matches!(
        crate::engine::DesktopEnvironment::detect(),
        crate::engine::DesktopEnvironment::Niri
    )
}

impl EffectsManager {
    /// Returns a wallpaper-derived spectrum palette. The cache key includes the
    /// wallpaper timestamp so replacing a file with the same name refreshes it.
    /// `top` selects which screen edge the palette should match.
    fn auto_palette(&mut self, wallpaper: &Path, top: bool) -> ([u8; 3], [u8; 3], [u8; 3]) {
        let key = (wallpaper.to_path_buf(), top);
        let stamp = std::fs::metadata(wallpaper)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0);

        if let Some((cached_stamp, palette)) = self.palette_cache.get(&key) {
            if *cached_stamp == stamp {
                return *palette;
            }
        }

        let palette = sample_palette(wallpaper, top).unwrap_or(FALLBACK_PALETTE);
        if self.palette_cache.len() >= 64 {
            self.palette_cache.clear();
        }
        self.palette_cache.insert(key, (stamp, palette));
        palette
    }

    /// Keeps the renderer alive while any effect is enabled. Effects are
    /// deliberately independent from video pause state: a paused wallpaper
    /// should not hide the spectrum, and a static wallpaper has no player to
    /// pause at all.
    pub fn sync(
        &mut self,
        config: &Config,
        outputs: &[MonitorOutput],
        paused_outputs: &HashSet<String>,
    ) -> Result<(), String> {
        if !config.parallax_enabled && !config.spectrum_enabled {
            self.stop();
            return Ok(());
        }
        if !renderer_supported() {
            self.stop();
            return Ok(());
        }

        let mut effect_outputs = Vec::new();
        for output in outputs.iter().filter(|output| output.name != "*") {
            let Some(path) = config.wallpaper_for(&output.name) else {
                continue;
            };
            let image_path = Path::new(path);
            let parallax = if config.parallax_enabled && is_static_image(image_path) {
                let depth = crate::depth::depth_map_path(image_path);
                depth.is_file().then(|| Parallax {
                    image: path.to_string(),
                    depth: depth.to_string_lossy().into_owned(),
                    strength: config.parallax_strength.clamp(0.1, 1.0),
                })
            } else {
                None
            };
            let spectrum = config.spectrum_enabled.then(|| {
                let (color1, color2, halo) = match config.spectrum_color_mode {
                    crate::config::SpectrumColorMode::Manual => {
                        let color1 = crate::config::parse_hex_color(&config.spectrum_color1)
                            .unwrap_or([120, 190, 255]);
                        let color2 = crate::config::parse_hex_color(&config.spectrum_color2)
                            .unwrap_or([196, 120, 255]);
                        (color1, color2, halo_for_colors(color1, color2))
                    }
                    crate::config::SpectrumColorMode::Auto => self.auto_palette(
                        image_path,
                        config.spectrum_position == crate::config::SpectrumPosition::Top,
                    ),
                };
                Spectrum {
                    bars: config.spectrum_bars.clamp(8, 64),
                    height_pct: config.spectrum_height_pct.clamp(4, 60),
                    color1,
                    color2,
                    halo,
                    alpha: config.spectrum_alpha.clamp(0.1, 1.0),
                    position: match config.spectrum_position {
                        crate::config::SpectrumPosition::Bottom => "bottom",
                        crate::config::SpectrumPosition::Top => "top",
                    },
                    style: match config.spectrum_style {
                        crate::config::SpectrumStyle::Bars => "bars",
                        crate::config::SpectrumStyle::Wave => "wave",
                        crate::config::SpectrumStyle::Dots => "dots",
                        crate::config::SpectrumStyle::Mirror => "mirror",
                    },
                    bar_width_pct: config.spectrum_bar_width_pct.clamp(10, 100),
                    glow: config.spectrum_glow,
                }
            });
            if parallax.is_some() || spectrum.is_some() {
                effect_outputs.push(EffectOutput {
                    name: output.name.clone(),
                    parallax,
                    spectrum,
                });
            }
        }

        if effect_outputs.is_empty() {
            // Nothing can be drawn yet (for example the depth map is still
            // being generated). Keep the manager idle instead of leaving a
            // stale renderer behind.
            self.stop();
            return Ok(());
        }

        self.desired_paused = paused_snapshot(paused_outputs)?;
        let config_text = serde_json::to_string(&EffectsConfig {
            outputs: effect_outputs,
        })
        .map_err(|error| error.to_string())?;

        if self.last_config.as_deref() == Some(config_text.as_str()) {
            if self.child_is_running() {
                return self.flush_control();
            }
            return self.start_child();
        }

        self.stop_renderer();
        let config_path =
            std::env::temp_dir().join(format!("aura-effects-{}.json", std::process::id()));
        std::fs::write(&config_path, &config_text).map_err(|error| error.to_string())?;
        self.config_path = Some(config_path);
        self.last_config = Some(config_text);
        self.start_child()
    }

    /// Recreates the layer-shell surfaces after a wallpaper is applied so they
    /// are created after `mpvpaper` and stay on top within the same layer.
    pub fn restart(&mut self) {
        if self.config_path.is_none() {
            return;
        }
        self.stop_renderer();
        if let Err(error) = self.start_child() {
            eprintln!("[Aura] Could not restart wallpaper effects: {error}");
        }
    }

    pub fn status(&mut self) -> EffectsStatus {
        if self.child_is_running() {
            return match &self.control_error {
                Some(error) => EffectsStatus::Failed(error.clone()),
                None => EffectsStatus::Running,
            };
        }
        match &self.last_error {
            Some(error) => EffectsStatus::Failed(error.clone()),
            None => EffectsStatus::Disabled,
        }
    }

    pub fn stop(&mut self) {
        self.stop_renderer();
        if let Some(path) = self.config_path.take() {
            let _ = std::fs::remove_file(path);
        }
        self.last_config = None;
        self.last_error = None;
        self.desired_paused.clear();
    }

    fn reset_control(&mut self) {
        self.control_stdin = None;
        self.last_sent_paused = None;
        self.control_pipe_buf = 0;
        self.control_error = None;
    }

    fn stop_renderer(&mut self) {
        self.reset_control();
        if let Some(mut child) = self.child.take() {
            if matches!(child.try_wait(), Ok(None)) {
                terminate_renderer(&mut child);
            }
            let _ = child.wait();
        }
    }

    fn child_is_running(&mut self) -> bool {
        let Some(child) = self.child.as_mut() else {
            return false;
        };
        match child.try_wait() {
            Ok(None) => true,
            Ok(Some(status)) => {
                self.child = None;
                self.reset_control();
                self.last_error = Some(format!(
                    "El renderizador terminó inesperadamente ({status}); revisa {}",
                    log_path().display()
                ));
                false
            }
            Err(error) => {
                self.child = None;
                self.reset_control();
                self.last_error = Some(format!("No se pudo consultar el renderizador: {error}"));
                false
            }
        }
    }

    fn fail_control(&mut self, error: io::Error) -> String {
        let message = format!("No se pudo controlar el paralaje: {error}");
        self.control_stdin = None;
        self.last_sent_paused = None;
        self.control_error = Some(message.clone());
        message
    }

    fn flush_control(&mut self) -> Result<(), String> {
        if let Some(error) = &self.control_error {
            return Err(error.clone());
        }
        if self.last_sent_paused.as_ref() == Some(&self.desired_paused) {
            return Ok(());
        }
        let result = match self.control_stdin.as_mut() {
            Some(stdin) => write_control(stdin, &self.desired_paused, self.control_pipe_buf),
            None => Err(io::Error::other("Control stdin unavailable")),
        };
        match result {
            Ok(true) => {
                self.last_sent_paused = Some(self.desired_paused.clone());
                Ok(())
            }
            Ok(false) => Ok(()),
            Err(error) => Err(self.fail_control(error)),
        }
    }

    fn start_child(&mut self) -> Result<(), String> {
        let config_path = self
            .config_path
            .as_ref()
            .ok_or_else(|| "Missing effects configuration".to_string())?;
        let helper = helper_path();
        let mut command = Command::new(&helper);
        command
            .arg("--config")
            .arg(config_path)
            .arg("--parent-pid")
            .arg(std::process::id().to_string())
            .arg("--control-stdin")
            .stdin(Stdio::piped())
            .stdout(Stdio::null());
        // Own process group: `cava` inherits it, so terminating the renderer
        // also reaps the audio visualizer instead of leaving it sampling.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                command.pre_exec(|| {
                    libc::setpgid(0, 0);
                    Ok(())
                });
            }
        }

        let log = logs_dir().and_then(|dir| {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("aura-effects.log"))
                .ok()
        });
        command.stderr(match log {
            Some(file) => Stdio::from(file),
            None => Stdio::null(),
        });

        match command.spawn() {
            Ok(mut child) => {
                self.reset_control();
                let stdin = child.stdin.take();
                self.child = Some(child);
                self.last_error = None;
                let setup = (|| {
                    let stdin = stdin.ok_or_else(|| io::Error::other("Missing control stdin"))?;
                    let fd = stdin.as_raw_fd();
                    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
                    if flags < 0
                        || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
                    {
                        return Err(io::Error::last_os_error());
                    }
                    let pipe_buf = unsafe { libc::fpathconf(fd, libc::_PC_PIPE_BUF) };
                    if pipe_buf <= 0 {
                        return Err(io::Error::other("Cannot determine control PIPE_BUF"));
                    }
                    Ok((stdin, pipe_buf as usize))
                })();
                match setup {
                    Ok((stdin, pipe_buf)) => {
                        self.control_stdin = Some(stdin);
                        self.control_pipe_buf = pipe_buf;
                        self.flush_control()
                    }
                    Err(error) => Err(self.fail_control(error)),
                }
            }
            Err(error) => {
                let message = format!(
                    "No se pudo iniciar el renderizador de efectos: {error} ({})",
                    helper.display()
                );
                self.last_error = Some(message.clone());
                Err(message)
            }
        }
    }
}

fn paused_snapshot(outputs: &HashSet<String>) -> Result<Vec<u8>, String> {
    if outputs.len() > 64 || outputs.iter().any(|name| name.is_empty() || name.len() > 256) {
        return Err("Invalid paused outputs: maximum 64 nonempty names of 256 bytes".into());
    }
    let mut names: Vec<&str> = outputs.iter().map(String::as_str).collect();
    names.sort_unstable();
    let mut message = serde_json::to_vec(&serde_json::json!({ "paused_outputs": names }))
        .map_err(|error| error.to_string())?;
    message.push(b'\n');
    if message.len() >= 4096 {
        return Err("Paused outputs control snapshot must be smaller than 4096 bytes".into());
    }
    Ok(message)
}

fn write_control(writer: &mut impl Write, message: &[u8], pipe_buf: usize) -> io::Result<bool> {
    if message.is_empty() || message.len() >= pipe_buf {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Control snapshot exceeds PIPE_BUF"));
    }
    // One atomic pipe write; EAGAIN keeps only the latest desired snapshot.
    match writer.write(message) {
        Ok(written) if written == message.len() => Ok(true),
        Ok(_) => Err(io::Error::new(io::ErrorKind::WriteZero, "Incomplete atomic control write")),
        Err(error) if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) => {
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

impl Drop for EffectsManager {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Sends SIGTERM to the renderer's process group so it can also stop cava,
/// then force-kills it if it does not exit promptly.
fn terminate_renderer(child: &mut Child) {
    #[cfg(unix)]
    {
        let pid = child.id() as i32;
        // The renderer was spawned with its own process group (pgid == pid).
        if unsafe { libc::kill(-pid, libc::SIGTERM) } != 0 {
            let _ = child.kill();
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
                Err(_) => break,
            }
        }
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
    let _ = child.kill();
}

fn logs_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?
        .join("aura")
        .join("logs");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir)
}

pub fn log_path() -> PathBuf {
    logs_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("aura-effects.log")
}

/// Locates `cava` the same way the renderer does: PATH first (so distro,
/// Flatpak and user installs win), then common prefixes used by GNOME, KDE and
/// minimal Wayland sessions that do not export `~/.local/bin`.
pub fn cava_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("cava");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let mut candidates = vec![
        PathBuf::from("/usr/bin/cava"),
        PathBuf::from("/usr/local/bin/cava"),
        PathBuf::from("/app/bin/cava"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join(".local/bin/cava"));
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

/// True when a `cava` executable is reachable. Without it the renderer still
/// runs, but the spectrum stays on its idle animation.
pub fn cava_available() -> bool {
    cava_binary().is_some()
}

fn helper_path() -> PathBuf {
    if let Ok(executable) = std::env::current_exe() {
        if let Some(sibling) = executable.parent().map(|parent| parent.join("aura-effects")) {
            if sibling.is_file() {
                return sibling;
            }
        }
    }
    PathBuf::from("aura-effects")
}

fn is_static_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            crate::scanner::IMAGE_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
        })
        .unwrap_or(false)
}

const FALLBACK_PALETTE: ([u8; 3], [u8; 3], [u8; 3]) =
    ([120, 190, 255], [196, 120, 255], [255, 255, 255]);

/// Derives two bright, saturated bar colors from the wallpaper while forcing
/// enough contrast against the area the strip actually covers. The halo color
/// is chosen opposite to the bars so they keep a visible edge on any backdrop.
pub(crate) fn palette_for_background(background: [u8; 3]) -> ([u8; 3], [u8; 3], [u8; 3]) {
    let (hue, saturation, _) = rgb_to_hsl(background);
    let luma = relative_luminance(background);
    let dark_background = luma < 0.38;
    let saturation = saturation.max(0.72);

    let start_lightness = if dark_background { 0.80 } else { 0.30 };
    let lifted = lift_contrast(
        hsl_to_rgb(hue, saturation, start_lightness),
        background,
        3.0,
        dark_background,
    );
    let color1 = if contrast_ratio(lifted, background) >= 3.0 {
        lifted
    } else {
        let flipped = lift_contrast(
            hsl_to_rgb(
                (hue + 180.0) % 360.0,
                saturation.max(0.80),
                if dark_background { 0.88 } else { 0.22 },
            ),
            background,
            3.0,
            dark_background,
        );
        if contrast_ratio(flipped, background) > contrast_ratio(lifted, background) {
            flipped
        } else {
            lifted
        }
    };

    let (color1_hue, color1_saturation, _) = rgb_to_hsl(color1);
    let color2 = lift_contrast(
        hsl_to_rgb(
            (color1_hue + 38.0) % 360.0,
            (color1_saturation + 0.08).min(0.95),
            if dark_background { 0.58 } else { 0.46 },
        ),
        background,
        2.4,
        dark_background,
    );

    let halo = halo_for_colors(color1, color2);
    (color1, color2, halo)
}

/// Moves HSL lightness away from the background luminance until the color
/// reaches `min_ratio` contrast. Some saturated hues (deep blue, violet) have
/// surprisingly low luminance, so a fixed lightness target is not enough.
fn lift_contrast(
    color: [u8; 3],
    background: [u8; 3],
    min_ratio: f32,
    dark_background: bool,
) -> [u8; 3] {
    let (hue, saturation, mut lightness) = rgb_to_hsl(color);
    let mut candidate = color;
    for _ in 0..32 {
        if contrast_ratio(candidate, background) >= min_ratio {
            break;
        }
        lightness = if dark_background {
            lightness + 0.03
        } else {
            lightness - 0.03
        };
        if !(0.0..=1.0).contains(&lightness) {
            break;
        }
        candidate = hsl_to_rgb(hue, saturation, lightness);
    }
    candidate
}

fn halo_for_colors(color1: [u8; 3], color2: [u8; 3]) -> [u8; 3] {
    let luma = (relative_luminance(color1) + relative_luminance(color2)) * 0.5;
    if luma > 0.45 {
        [6, 6, 10]
    } else {
        [255, 255, 255]
    }
}

fn sample_palette(wallpaper: &Path, top: bool) -> Option<([u8; 3], [u8; 3], [u8; 3])> {
    use image::GenericImageView;
    let source = if is_static_image(wallpaper) {
        wallpaper.to_path_buf()
    } else {
        let thumbnail = crate::scanner::thumbs::thumb_path_for_video(wallpaper);
        if !thumbnail.is_file() {
            return None;
        }
        thumbnail
    };

    let image = image::open(&source).ok()?.thumbnail(64, 36);
    let (width, height) = (image.width().max(1), image.height().max(1));
    let (start_row, end_row) = if top {
        (0, ((height as f32) * 0.45) as u32)
    } else {
        (((height as f32) * 0.55) as u32, height)
    };
    let mut sum = [0u64; 3];
    let mut count = 0u64;
    for y in start_row..end_row {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            sum[0] += pixel[0] as u64;
            sum[1] += pixel[1] as u64;
            sum[2] += pixel[2] as u64;
            count += 1;
        }
    }
    if count == 0 {
        return None;
    }
    let background = [
        (sum[0] / count) as u8,
        (sum[1] / count) as u8,
        (sum[2] / count) as u8,
    ];
    Some(palette_for_background(background))
}

fn rgb_to_hsl([red, green, blue]: [u8; 3]) -> (f32, f32, f32) {
    let r = red as f32 / 255.0;
    let g = green as f32 / 255.0;
    let b = blue as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = (max + min) * 0.5;
    let delta = max - min;
    if delta <= f32::EPSILON {
        return (0.0, 0.0, lightness);
    }
    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
    let hue = if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (hue.rem_euclid(360.0), saturation, lightness)
}

fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> [u8; 3] {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue = hue.rem_euclid(360.0) / 60.0;
    let x = chroma * (1.0 - (hue % 2.0 - 1.0).abs());
    let (r, g, b) = match hue as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma * 0.5;
    let channel = |value: f32| ((value + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    [channel(r), channel(g), channel(b)]
}

fn channel_luminance(value: u8) -> f32 {
    let channel = value as f32 / 255.0;
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance([red, green, blue]: [u8; 3]) -> f32 {
    0.2126 * channel_luminance(red)
        + 0.7152 * channel_luminance(green)
        + 0.0722 * channel_luminance(blue)
}

fn contrast_ratio(foreground: [u8; 3], background: [u8; 3]) -> f32 {
    let a = relative_luminance(foreground) + 0.05;
    let b = relative_luminance(background) + 0.05;
    a.max(b) / a.min(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_palette_stays_visible_on_any_background() {
        let backgrounds = [
            [8, 10, 16],
            [36, 32, 48],
            [90, 92, 96],
            [220, 226, 240],
            [246, 240, 228],
            [24, 70, 130],
            [210, 160, 40],
            [150, 30, 40],
        ];
        for background in backgrounds {
            let (color1, color2, halo) = palette_for_background(background);
            assert!(
                contrast_ratio(color1, background) >= 3.0,
                "base color {color1:?} too weak over {background:?}"
            );
            assert!(
                contrast_ratio(color2, background) >= 2.4,
                "tip color {color2:?} too weak over {background:?}"
            );

            let bars_luma = (relative_luminance(color1) + relative_luminance(color2)) * 0.5;
            let expected_halo = if bars_luma > 0.45 {
                [6, 6, 10]
            } else {
                [255, 255, 255]
            };
            assert_eq!(halo, expected_halo);
        }
    }

    #[test]
    fn hsl_roundtrip_preserves_primary_colors() {
        for rgb in [[255u8, 0, 0], [0, 255, 0], [0, 0, 255]] {
            let (hue, saturation, lightness) = rgb_to_hsl(rgb);
            assert!(saturation > 0.99 && (lightness - 0.5).abs() < 0.01);
            assert_eq!(hsl_to_rgb(hue, saturation, lightness), rgb);
        }
    }
}
