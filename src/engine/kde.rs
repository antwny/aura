//! KDE Plasma backend.
//!
//! KWin deliberately keeps Plasma's desktop containment above wlr-layer-shell
//! surfaces, so mpvpaper cannot be used as a video wallpaper on Plasma. Aura
//! therefore talks to Plasma's own wallpaper plugin over D-Bus. Videos use the
//! Smart Video Wallpaper Reborn plugin; static images use the stock image
//! plugin and keep working even when the video plugin is not installed.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::config::AutoPauseMode;

pub const PLUGIN_ID: &str = "luisbocanegra.smart.video.wallpaper.reborn";
pub const PLUGIN_URL: &str = "https://github.com/luisbocanegra/plasma-smart-video-wallpaper-reborn";
const PLASMA_SERVICE: &str = "org.kde.plasmashell";
const PLASMA_PATH: &str = "/PlasmaShell";
const PLASMA_INTERFACE: &str = "org.kde.PlasmaShell";
const IMAGE_PLUGIN_ID: &str = "org.kde.image";

fn data_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    let system = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    std::iter::once(data_home)
        .chain(system.split(':').filter(|d| !d.is_empty()).map(PathBuf::from))
        .collect()
}

fn plugin_dirs() -> Vec<PathBuf> {
    data_dirs()
        .into_iter()
        .map(|dir| dir.join("plasma/wallpapers").join(PLUGIN_ID))
        .collect()
}

pub fn plugin_installed() -> bool {
    plugin_dirs().iter().any(|dir| {
        dir.join("metadata.json").is_file() || dir.join("contents/ui/main.qml").is_file()
    })
}

pub fn plugin_install_hint() -> String {
    format!(
        "Smart Video Wallpaper Reborn is not installed. Install it from KDE System Settings → Wallpaper → Get New Plugins, or from {}",
        PLUGIN_URL
    )
}

fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for byte in path.to_string_lossy().bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                uri.push(byte as char)
            }
            _ => uri.push_str(&format!("%{:02X}", byte)),
        }
    }
    uri
}

fn js_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
}

fn plasma_command() -> Option<&'static str> {
    ["qdbus6", "qdbus-qt6", "qdbus"]
        .into_iter()
        .find(|candidate| {
            Command::new(candidate)
                .arg("--version")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
        })
}

/// True when a Qt D-Bus CLI, gdbus, or busctl is available.
pub fn dbus_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        plasma_command().is_some()
            || Command::new("gdbus")
                .arg("--version")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            || Command::new("busctl")
                .arg("--version")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
    })
}

/// Runs a Plasma scripting snippet. Plasma's evaluateScript is synchronous and
/// returns before the desktop has finished repainting, which is enough for us:
/// the containment owns the actual video lifecycle after that point.
pub fn evaluate_script(script: &str) -> Result<(), String> {
    // 1. Try qdbus / qdbus6 with full interface method name
    if let Some(command) = plasma_command() {
        let output = Command::new(command)
            .args([
                PLASMA_SERVICE,
                PLASMA_PATH,
                &format!("{}.evaluateScript", PLASMA_INTERFACE),
                script,
            ])
            .stdin(Stdio::null())
            .output();

        if let Ok(ref out) = output {
            if out.status.success() {
                return Ok(());
            }
        }

        // 2. Try qdbus without interface prefix (some Qt6/Plasma6 builds match bare method name)
        let output2 = Command::new(command)
            .args([PLASMA_SERVICE, PLASMA_PATH, "evaluateScript", script])
            .stdin(Stdio::null())
            .output();

        if let Ok(ref out) = output2 {
            if out.status.success() {
                return Ok(());
            }
        }
    }

    // 3. Fallback: try gdbus (direct D-Bus method call without Qt introspection)
    let gdbus_output = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            PLASMA_SERVICE,
            "--object-path",
            PLASMA_PATH,
            "--method",
            &format!("{}.evaluateScript", PLASMA_INTERFACE),
            script,
        ])
        .stdin(Stdio::null())
        .output();

    if let Ok(ref out) = gdbus_output {
        if out.status.success() {
            return Ok(());
        }
    }

    // 4. Fallback: try busctl (systemd D-Bus CLI, universally installed on Arch/CachyOS/systemd)
    let busctl_output = Command::new("busctl")
        .args([
            "--user",
            "call",
            PLASMA_SERVICE,
            PLASMA_PATH,
            PLASMA_INTERFACE,
            "evaluateScript",
            "s",
            script,
        ])
        .stdin(Stdio::null())
        .output();

    if let Ok(ref out) = busctl_output {
        if out.status.success() {
            return Ok(());
        }
    }

    // If all failed, gather error output from qdbus or available tool
    if let Some(command) = plasma_command() {
        let output = Command::new(command)
            .args([
                PLASMA_SERVICE,
                PLASMA_PATH,
                &format!("{}.evaluateScript", PLASMA_INTERFACE),
                script,
            ])
            .stdin(Stdio::null())
            .output()
            .map_err(|error| format!("Could not run {}: {}", command, error))?;

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let err = if stderr.is_empty() { stdout } else { stderr };
        return Err(if err.is_empty() {
            "Plasma scripting failed (unknown error)".into()
        } else {
            err
        });
    }

    Err("Neither qdbus6, qdbus, gdbus, nor busctl is available to communicate with KDE Plasma.".into())
}

/// Smart Video Wallpaper Reborn expects `VideoUrls` to be a JSON array of
/// objects (not a plain list of strings). Its parser filters entries where
/// `enabled` is falsy, so a string-only array produces a black desktop even
/// though the plugin is installed and selected.
fn video_urls(path: &Path) -> String {
    serde_json::json!([{
        "filename": file_uri(path),
        "enabled": true,
        "duration": 0,
        "customDuration": 0,
        "playbackRate": 0.0,
        "alternativePlaybackRate": 0.0,
        "loop": false,
        "dayNightCycleAssignment": 0,
    }])
    .to_string()
}

/// Check if /PlasmaShell object is responsive without crawling the whole hierarchy.
fn is_plasmashell_responsive() -> bool {
    if let Some(command) = plasma_command() {
        if Command::new(command)
            .args([PLASMA_SERVICE, PLASMA_PATH])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return true;
        }
    }
    // Fallback: check via busctl
    if Command::new("busctl")
        .args(["--user", "introspect", PLASMA_SERVICE, PLASMA_PATH])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return true;
    }
    // Fallback: check via gdbus
    if Command::new("gdbus")
        .args(["introspect", "--session", "--dest", PLASMA_SERVICE, "--object-path", PLASMA_PATH])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return true;
    }
    false
}

/// Autostart can launch Aura before plasmashell has finished creating its
/// desktop containments. Wait briefly (once) for /PlasmaShell to be responsive.
fn wait_for_plasmashell() {
    static READY: AtomicBool = AtomicBool::new(false);
    if READY.load(Ordering::Relaxed) {
        return;
    }

    // Fast path: if plasmashell is already responsive, avoid any sleep or looping
    if is_plasmashell_responsive() {
        READY.store(true, Ordering::Relaxed);
        return;
    }

    // Slow path (only during early session autostart before plasmashell starts):
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(250));
        if is_plasmashell_responsive() {
            // Desktops are created shortly after the service appears.
            std::thread::sleep(Duration::from_millis(300));
            READY.store(true, Ordering::Relaxed);
            return;
        }
    }

    // Mark ready so subsequent manual user clicks do not block repeatedly
    READY.store(true, Ordering::Relaxed);
}

fn screen_index_for_output(output: &str) -> Option<usize> {
    if output == "*" || output.is_empty() {
        return None;
    }
    crate::engine::detect_outputs()
        .iter()
        .position(|monitor| monitor.name == output)
}

fn target_guard(output: &str) -> String {
    match screen_index_for_output(output) {
        Some(index) => format!("if (all[i].screen !== {}) continue;", index),
        None => String::new(),
    }
}

fn desktop_loop(output: &str, body: &str) -> String {
    format!(
        "var all = desktops();\nfor (var i = 0; i < all.length; i++) {{\n  {} \n  var d = all[i];\n{}\n}}\n",
        target_guard(output),
        body
    )
}

fn image_fill_mode(scaling: &str) -> u32 {
    // Plasma image wallpaper FillMode: 0 stretch, 1 preserve fit, 2 preserve crop.
    match scaling {
        "stretch" => 0,
        "fill" => 2,
        _ => 1,
    }
}

fn video_fill_mode(scaling: &str) -> u32 {
    match scaling {
        "stretch" => 0,
        "fit" => 1,
        _ => 2,
    }
}

fn plugin_config_group() -> String {
    format!(
        "d.currentConfigGroup = new Array(\"Wallpaper\", \"{}\", \"General\");",
        PLUGIN_ID
    )
}

/// Applies a still image to a single output when possible. If Plasma's screen
/// numbering cannot be matched, all desktops are updated as a safe fallback.
pub fn set_image_on_output(output: &str, path: &Path, scaling: &str) -> Result<(), String> {
    wait_for_plasmashell();
    let uri = js_string(&file_uri(path));
    let fill_mode = image_fill_mode(scaling);
    let body = format!(
        "  d.wallpaperPlugin = \"{}\";\n  d.currentConfigGroup = new Array(\"Wallpaper\", \"{}\", \"General\");\n  d.writeConfig(\"Image\", {});\n  d.writeConfig(\"FillMode\", {});",
        IMAGE_PLUGIN_ID, IMAGE_PLUGIN_ID, uri, fill_mode
    );
    evaluate_script(&desktop_loop(output, &body))
}

pub fn set_video_on_output(
    output: &str,
    path: &Path,
    scaling: &str,
    mute: bool,
    volume: u8,
    auto_pause: bool,
    auto_pause_mode: AutoPauseMode,
) -> Result<(), String> {
    if !plugin_installed() {
        return Err(plugin_install_hint());
    }
    wait_for_plasmashell();
    let urls = js_string(&video_urls(path));
    let fill_mode = video_fill_mode(scaling);
    let mute_mode = if mute { 5 } else { 4 };
    let volume = (volume as f32 / 100.0).clamp(0.0, 1.0);
    let pause_mode = auto_pause_mode.kde_pause_mode(auto_pause);
    let body = format!(
        "  d.wallpaperPlugin = \"{}\";\n  {}\n  d.writeConfig(\"VideoUrls\", {});\n  d.writeConfig(\"FillMode\", {});\n  d.writeConfig(\"MuteMode\", {});\n  d.writeConfig(\"Volume\", {});\n  d.writeConfig(\"PauseMode\", {});\n  d.reloadConfig();",
        PLUGIN_ID,
        plugin_config_group(),
        urls,
        fill_mode,
        mute_mode,
        volume,
        pause_mode
    );
    evaluate_script(&desktop_loop(output, &body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_string_escapes_quotes_and_slashes() {
        assert_eq!(js_string("a/b"), "\"a/b\"");
        assert_eq!(js_string("a\"b"), "\"a\\\"b\"");
    }

    #[test]
    fn video_urls_use_plugin_object_schema() {
        let encoded = video_urls(Path::new("/a/b c.mkv"));
        let value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        let entry = &value[0];
        assert_eq!(entry["filename"], "file:///a/b%20c.mkv");
        assert_eq!(entry["enabled"], true);
        assert_eq!(entry["duration"], 0);
        assert_eq!(entry["customDuration"], 0);
        assert_eq!(entry["playbackRate"], 0.0);
        assert_eq!(entry["alternativePlaybackRate"], 0.0);
        assert_eq!(entry["loop"], false);
        assert_eq!(entry["dayNightCycleAssignment"], 0);
    }

    #[test]
    fn file_uri_encodes_spaces() {
        assert_eq!(file_uri(Path::new("/a/b c.mkv")), "file:///a/b%20c.mkv");
    }

    #[test]
    fn fill_modes_match_plasma_contract() {
        assert_eq!(image_fill_mode("fill"), 2);
        assert_eq!(image_fill_mode("fit"), 1);
        assert_eq!(image_fill_mode("stretch"), 0);
        assert_eq!(video_fill_mode("fill"), 2);
        assert_eq!(video_fill_mode("fit"), 1);
        assert_eq!(video_fill_mode("stretch"), 0);
    }

    #[test]
    fn desktop_loop_generates_valid_javascript_structure() {
        let script = desktop_loop("*", "d.wallpaperPlugin = \"test\";");
        assert!(script.contains("var all = desktops();"));
        assert!(script.contains("for (var i = 0; i < all.length; i++)"));
        assert!(script.contains("d.wallpaperPlugin = \"test\";"));
    }

    #[test]
    fn wait_for_plasmashell_terminates_promptly() {
        // Must never hang or loop infinitely even when plasmashell is not present
        let start = std::time::Instant::now();
        wait_for_plasmashell();
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
    }
}
