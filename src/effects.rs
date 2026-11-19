//! Lifecycle manager for Aura's independent Wayland effects renderer.

use crate::config::Config;
use crate::engine::outputs::MonitorOutput;
use serde::Serialize;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

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
    alpha: f32,
}

impl EffectsManager {
    /// Keeps the renderer alive while any effect is enabled. Effects are
    /// deliberately independent from video pause state: a paused wallpaper
    /// should not hide the spectrum, and a static wallpaper has no player to
    /// pause at all.
    pub fn sync(&mut self, config: &Config, outputs: &[MonitorOutput]) -> Result<(), String> {
        if !config.parallax_enabled && !config.spectrum_enabled {
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
            let spectrum = config.spectrum_enabled.then(|| Spectrum {
                bars: config.spectrum_bars.clamp(8, 64),
                height_pct: config.spectrum_height_pct.clamp(5, 40),
                color1: [120, 190, 255],
                color2: [196, 120, 255],
                alpha: config.spectrum_alpha.clamp(0.1, 1.0),
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

        let config_text = serde_json::to_string(&EffectsConfig {
            outputs: effect_outputs,
        })
        .map_err(|error| error.to_string())?;

        if self.last_config.as_deref() == Some(config_text.as_str()) {
            if self.child_is_running() {
                return Ok(());
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
            return EffectsStatus::Running;
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
    }

    fn stop_renderer(&mut self) {
        if let Some(mut child) = self.child.take() {
            if matches!(child.try_wait(), Ok(None)) {
                let _ = child.kill();
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
                self.last_error = Some(format!(
                    "El renderizador terminó inesperadamente ({status}); revisa {}",
                    log_path().display()
                ));
                false
            }
            Err(error) => {
                self.child = None;
                self.last_error = Some(format!("No se pudo consultar el renderizador: {error}"));
                false
            }
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
            .stdin(Stdio::null())
            .stdout(Stdio::null());

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
            Ok(child) => {
                self.child = Some(child);
                self.last_error = None;
                Ok(())
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

impl Drop for EffectsManager {
    fn drop(&mut self) {
        self.stop();
    }
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
