use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Desktop integration facts collected once when Aura starts. Detecting these
/// per frame would be expensive and could block rendering.
#[derive(Debug, Clone, Default)]
pub struct IntegrationInfo {
    pub niri: bool,
    pub niri_socket: bool,
    pub dms: bool,
    pub quickshell: bool,
    pub dms_underlay: bool,
    pub niri_version: Option<String>,
    pub dms_version: Option<String>,
}

pub fn detect() -> IntegrationInfo {
    let niri_socket = std::env::var_os("NIRI_SOCKET").is_some();
    let (niri, niri_version) = command_version("niri", &["--version"]);
    let (dms, dms_version) = command_version("dms", &["version"]);
    let quickshell = command_exists("quickshell")
        || process_running("quickshell")
        || process_running("dms");

    IntegrationInfo {
        niri: niri || niri_socket,
        niri_socket,
        dms,
        quickshell,
        // DMS does not currently expose a stable set-wallpaper IPC. Keep this
        // false unless a known endpoint appears; the setting remains visible
        // as an explicit best-effort/diagnostic option.
        dms_underlay: dms && dms_supports_wallpaper(),
        niri_version,
        dms_version,
    }
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", name))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn command_version(name: &str, args: &[&str]) -> (bool, Option<String>) {
    let output = match Command::new(name).args(args).output() {
        Ok(output) => output,
        Err(_) => return (false, None),
    };
    if !output.status.success() {
        return (false, None);
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (true, (!text.is_empty()).then_some(text))
}

fn process_running(name: &str) -> bool {
    let output = Command::new("pgrep")
        .arg("-x")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    output.map(|s| s.success()).unwrap_or(false)
}

fn dms_supports_wallpaper() -> bool {
    let output = match Command::new("dms").args(["ipc", "list"]).output() {
        Ok(output) if output.status.success() => output,
        _ => return false,
    };
    let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
    text.contains("setwallpaper") || text.contains("wallpaper set") || text.contains("wallpaper.set")
}

/// Returns an image suitable for a compositor/DMS underlay. Videos use their
/// generated poster when available; images use the original file.
pub fn underlay_image_for(video_path: &Path) -> Option<PathBuf> {
    let ext = video_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if crate::scanner::IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        return video_path.is_file().then(|| video_path.to_path_buf());
    }
    let thumb = crate::scanner::thumbs::thumb_path_for_video(video_path);
    thumb.is_file().then_some(thumb)
}

/// Best-effort DMS underlay sync. This is intentionally optional: if the
/// installed DMS version has no stable IPC endpoint we return false and Aura's
/// transactional mpvpaper transition remains the only visible layer.
pub fn sync_dms_underlay(path: &Path) -> bool {
    if !command_exists("dms") {
        return false;
    }
    let path = path.to_string_lossy().to_string();
    let attempts: [(&str, &str); 2] = [
        ("settings", "set"),
        ("session", "setWallpaper"),
    ];
    for (target, function) in attempts {
        let mut command = Command::new("dms");
        command.args(["ipc", "call", target, function]);
        if function == "set" {
            command.args(["wallpaperPath", &path]);
        } else {
            command.arg(&path);
        }
        let Ok(mut child) = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            continue;
        };
        let deadline = Instant::now() + Duration::from_millis(900);
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => return true,
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
            }
        }
    }
    false
}

pub fn niri_layer_rule() -> &'static str {
    "layer-rule {\n    match namespace=\"^mpvpaper$\"\n    place-within-backdrop true\n}\n"
}

pub fn niri_window_rule() -> &'static str {
    "// Optional: remember Aura's preferred size on Niri. Position is compositor-owned.\nwindow-rule {\n    match app-id=\"io.github.antwny.aura\"\n    default-column-width { proportion 0.55; }\n    default-window-height { proportion 0.75; }\n}\n"
}

pub fn niri_shortcuts() -> &'static str {
    "binds {\n    Mod+Shift+W { spawn \"aura\" \"switcher\"; }\n    Mod+Shift+P { spawn \"aura\" \"toggle-pause\"; }\n    Mod+Shift+S { spawn \"aura\" \"stop\"; }\n}\n"
}

pub fn niri_autostart() -> &'static str {
    "# Aura must start after NIRI_SOCKET exists and, if used, after DMS/Quickshell.\n# In ~/.config/niri/config.kdl:\nspawn-at-startup \"sh\" \"-lc\" \"sleep 2; exec aura --hidden\"\n\n# More reliable user service:\n# ~/.config/systemd/user/aura.service\n# [Unit]\n# Description=Aura live wallpaper daemon\n# After=graphical-session.target niri.service\n# PartOf=graphical-session.target\n# [Service]\n# ExecStart=%h/.local/bin/aura --hidden\n# Restart=on-failure\n# [Install]\n# WantedBy=graphical-session.target\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_rule_contains_namespace() {
        assert!(niri_layer_rule().contains("mpvpaper"));
        assert!(niri_layer_rule().contains("place-within-backdrop"));
    }
}
