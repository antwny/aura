use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

pub fn resolve_mpvpaper_binary() -> std::ffi::OsString {
    // 1. Check if mpvpaper exists alongside current_exe
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let sibling = parent.join("mpvpaper");
            if sibling.is_file() {
                return sibling.into_os_string();
            }
        }
    }
    // 2. Check ~/.local/bin/mpvpaper
    if let Ok(home) = std::env::var("HOME") {
        let p = std::path::PathBuf::from(home).join(".local/bin/mpvpaper");
        if p.is_file() {
            return p.into_os_string();
        }
    }
    // 3. Check /usr/local/bin/mpvpaper
    let p = std::path::Path::new("/usr/local/bin/mpvpaper");
    if p.is_file() {
        return p.into();
    }
    // 4. Check /usr/bin/mpvpaper
    let p = std::path::Path::new("/usr/bin/mpvpaper");
    if p.is_file() {
        return p.into();
    }
    "mpvpaper".into()
}

pub struct WallpaperEngine {
    // Stores active process handle per output
    processes: HashMap<String, Child>,
    pub is_paused: bool,
}

impl WallpaperEngine {
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
            is_paused: false,
        }
    }

    pub fn set_wallpaper(
        &mut self,
        output: &str,
        video_path: &str,
        scaling: &str,
        mute: bool,
        volume: u8,
        hwdec: &str,
    ) -> Result<u32, std::io::Error> {
        // If applying to all monitors ("*"), stop any specific outputs first
        if output == "*" {
            self.stop_all();
        } else {
            // Stop wildcard if running, and stop this specific output's previous process
            self.stop_output("*");
            self.stop_output(output);
        }

        let is_image = std::path::Path::new(video_path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| crate::scanner::IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false);

        // Pre-scale large static images to screen resolution to prevent 400MB+ RAM and 1.2GB GTT usage
        let effective_path = if is_image {
            crate::scanner::thumbs::optimize_wallpaper_image(std::path::Path::new(video_path), 2560, 1440)
        } else {
            std::path::PathBuf::from(video_path)
        };
        let effective_path_str = effective_path.to_string_lossy().to_string();

        let opts = Self::build_mpv_options(scaling, mute, volume, hwdec, is_image);

        let child = match Command::new(resolve_mpvpaper_binary())
            .arg("-l")
            .arg("bottom")
            .arg("-o")
            .arg(&opts)
            .arg(output)
            .arg(&effective_path_str)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[Aura Engine] Error al iniciar mpvpaper: {} (output: {}, video: {})", e, output, video_path);
                    return Err(e);
                }
            };

        let pid = child.id();
        self.processes.insert(output.to_string(), child);
        self.is_paused = false;
        Ok(pid)
    }

    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        !self.processes.is_empty()
    }

    pub fn toggle_pause(&mut self) -> bool {
        if self.is_paused {
            self.resume_all();
            false
        } else {
            self.pause_all();
            true
        }
    }

    pub fn pause_all(&mut self) {
        // Send SIGSTOP to freeze video playback and reduce GPU/CPU load to 0% via POSIX signal
        for (_, child) in &self.processes {
            let pid = child.id() as i32;
            unsafe {
                libc::kill(pid, libc::SIGSTOP);
            }
        }
        self.is_paused = true;
    }

    pub fn resume_all(&mut self) {
        // Send SIGCONT to smoothly unfreeze playback via POSIX signal
        for (_, child) in &self.processes {
            let pid = child.id() as i32;
            unsafe {
                libc::kill(pid, libc::SIGCONT);
            }
        }
        self.is_paused = false;
    }

    pub fn stop_output(&mut self, output: &str) {
        if let Some(mut child) = self.processes.remove(output) {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn stop_all(&mut self) {
        for (_, mut child) in self.processes.drain() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.is_paused = false;
    }

    pub fn build_mpv_options(scaling: &str, mute: bool, volume: u8, hwdec: &str, is_image: bool) -> String {
        let mut opts = if is_image {
            String::from("image-display-duration=inf --pause=yes --no-config --no-audio --demuxer-max-bytes=8M --vd-lavc-threads=1")
        } else {
            let mut o = format!(
                "loop-file=inf --hwdec={} --no-config --demuxer-max-bytes=24M --demuxer-readahead-secs=2 --vd-lavc-threads=2 --background-color=#000000",
                hwdec
            );
            if mute || volume == 0 {
                o.push_str(" --no-audio");
            } else {
                o.push_str(&format!(" --volume={}", volume.min(100)));
            }
            o
        };

        match scaling {
            "fill" => opts.push_str(" --panscan=1.0"),
            "stretch" => opts.push_str(" --no-keepaspect"),
            _ => {} // fit is default
        }
        opts
    }

    pub fn autostart_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(".config").join("autostart")
    }

    pub fn autostart_file() -> PathBuf {
        Self::autostart_dir().join("io.github.antwny.aura.desktop")
    }

    pub fn write_autostart() -> std::io::Result<()> {
        let dir = Self::autostart_dir();
        std::fs::create_dir_all(&dir)?;

        let is_sandboxed = std::path::Path::new("/.flatpak-info").exists()
            || std::env::var_os("FLATPAK_ID").is_some()
            || std::env::var_os("SNAP").is_some();

        let (exec_cmd, try_exec) = if is_sandboxed {
            ("flatpak run io.github.antwny.aura --hidden".to_string(), "flatpak".to_string())
        } else {
            let home = std::env::var("HOME").unwrap_or_default();
            let user_bin = PathBuf::from(&home).join(".local/bin/aura");
            if user_bin.exists() {
                (format!("{} --hidden", user_bin.display()), user_bin.display().to_string())
            } else if let Ok(exe) = std::env::current_exe() {
                (format!("{} --hidden", exe.display()), exe.display().to_string())
            } else {
                ("aura --hidden".to_string(), "aura".to_string())
            }
        };

        let desktop_content = format!(
            "[Desktop Entry]\n\
            Type=Application\n\
            Name=Aura\n\
            GenericName=Live Wallpaper Manager\n\
            Comment=Animated live wallpaper manager for COSMIC Desktop\n\
            TryExec={}\n\
            Exec={}\n\
            Icon=io.github.antwny.aura\n\
            Terminal=false\n\
            StartupNotify=false\n\
            X-GNOME-Autostart-enabled=true\n\
            X-Cosmic-Autostart-enabled=true\n\
            X-GNOME-Autostart-Delay=2\n\
            Categories=Utility;DesktopSettings;\n",
            try_exec,
            exec_cmd
        );

        std::fs::write(Self::autostart_file(), desktop_content)?;
        Ok(())
    }

    pub fn remove_autostart() -> std::io::Result<()> {
        let file = Self::autostart_file();
        if file.exists() {
            std::fs::remove_file(file)?;
        }
        Ok(())
    }

    pub fn is_autostart_enabled() -> bool {
        Self::autostart_file().exists()
    }
}

#[allow(dead_code)]
pub fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_mpv_options() {
        let opts_fit_mute = WallpaperEngine::build_mpv_options("fit", true, 100, "auto-safe", false);
        assert!(opts_fit_mute.contains("--hwdec=auto-safe"));
        assert!(opts_fit_mute.contains("--no-audio"));
        assert!(opts_fit_mute.contains("--demuxer-max-bytes=24M"));
        assert!(opts_fit_mute.contains("--vd-lavc-threads=2"));
        assert!(opts_fit_mute.contains("--background-color=#000000"));
        assert!(!opts_fit_mute.contains("--panscan"));

        let opts_fill_sound = WallpaperEngine::build_mpv_options("fill", false, 80, "vaapi", false);
        assert!(opts_fill_sound.contains("--hwdec=vaapi"));
        assert!(!opts_fill_sound.contains("--no-audio"));
        assert!(opts_fill_sound.contains("--volume=80"));
        assert!(opts_fill_sound.contains("--panscan=1.0"));

        let opts_stretch = WallpaperEngine::build_mpv_options("stretch", false, 100, "nvdec", false);
        assert!(opts_stretch.contains("--no-keepaspect"));

        let opts_image = WallpaperEngine::build_mpv_options("fill", true, 100, "auto-safe", true);
        assert!(opts_image.contains("image-display-duration=inf"));
        assert!(opts_image.contains("--pause=yes"));
        assert!(opts_image.contains("--demuxer-max-bytes=8M"));
        assert!(opts_image.contains("--vd-lavc-threads=1"));
        assert!(opts_image.contains("--panscan=1.0"));
    }

    #[test]
    fn test_shell_escape_safety() {
        assert_eq!(shell_escape("normal_string"), "'normal_string'");
        assert_eq!(shell_escape("with space"), "'with space'");
        assert_eq!(shell_escape("with$dollar"), "'with$dollar'");
        assert_eq!(shell_escape("with`backtick`"), "'with`backtick`'");
        assert_eq!(shell_escape("with'quote"), "'with'\\''quote'");
    }
}
