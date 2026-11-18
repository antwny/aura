//! Cross-desktop session autostart.
//!
//! The primary mechanism is the freedesktop XDG autostart entry in
//! `~/.config/autostart`, processed by COSMIC, GNOME, Plasma, XFCE, Cinnamon,
//! MATE, LXQt, and by niri-session/uwsm through systemd's xdg-autostart
//! generator.
//!
//! Compositors launched directly without a session manager (Hyprland, Sway,
//! other Wayland sessions) do not read that directory. For those Aura installs
//! a systemd user service instead, with a small launcher that waits for the
//! compositor socket and exports variables the compositor does not forward to
//! the systemd user manager.
//!
//! Exactly one mechanism is enabled at a time: two simultaneous launches would
//! activate the main window at login (the second instance talks to the first
//! over D-Bus and shows it).

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const APP_ID: &str = "io.github.antwny.aura";
const UNIT_NAME: &str = "aura.service";

/// True when either mechanism currently starts Aura with the session.
pub fn is_enabled() -> bool {
    xdg_enabled() || systemd_enabled()
}

/// Enables autostart with the mechanism that fits the running session.
pub fn enable() -> io::Result<()> {
    if use_systemd_unit() {
        // Keep a single mechanism so login never launches Aura twice.
        let _ = remove_xdg_entry();
        write_systemd_unit()
    } else {
        remove_systemd_unit();
        write_xdg_entry()
    }
}

/// Removes every autostart mechanism, so switching installations or desktops
/// cannot leave a stale entry behind.
pub fn disable() -> io::Result<()> {
    remove_systemd_unit();
    remove_xdg_entry()
}

fn is_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
        || std::env::var_os("FLATPAK_ID").is_some()
}

fn is_snap() -> bool {
    std::env::var_os("SNAP").is_some()
}

fn is_sandboxed() -> bool {
    is_flatpak() || is_snap()
}

fn use_systemd_unit() -> bool {
    !is_sandboxed() && systemd_user_available() && !session_handles_xdg_autostart()
}

/// Whether this session processes `~/.config/autostart` on its own.
fn session_handles_xdg_autostart() -> bool {
    // uwsm starts xdg-desktop-autostart.target for compositors like Hyprland.
    if std::env::var_os("UWSM_FINALIZE_VARIABLES").is_some()
        || std::env::var_os("UWSM_WAIT_VARNAMES").is_some()
    {
        return true;
    }
    // niri-session starts it through niri.service, and it stays active for the
    // whole session. On GNOME/Plasma the desktop reads the entries itself.
    if systemctl_ok(&["is-active", "xdg-desktop-autostart.target"]) {
        return true;
    }
    matches!(
        crate::engine::DesktopEnvironment::detect(),
        crate::engine::DesktopEnvironment::Cosmic
            | crate::engine::DesktopEnvironment::Gnome
            | crate::engine::DesktopEnvironment::Kde
            | crate::engine::DesktopEnvironment::Xfce
            | crate::engine::DesktopEnvironment::Cinnamon
            | crate::engine::DesktopEnvironment::Mate
            | crate::engine::DesktopEnvironment::Lxqt
            | crate::engine::DesktopEnvironment::Budgie
            | crate::engine::DesktopEnvironment::Deepin
            | crate::engine::DesktopEnvironment::Unity
    )
}

fn systemd_user_available() -> bool {
    Command::new("systemctl")
        .args(["--user", "show-environment"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn systemctl_ok(args: &[&str]) -> bool {
    Command::new("systemctl")
        .arg("--user")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn systemctl(args: &[&str]) -> bool {
    systemctl_ok(args)
}

fn config_home() -> PathBuf {
    crate::config::config_home()
}

fn data_home() -> PathBuf {
    #[cfg(test)]
    crate::test_support::init();

    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".local/share")
}

// ---------------------------------------------------------------------------
// XDG autostart
// ---------------------------------------------------------------------------

fn xdg_autostart_dir() -> PathBuf {
    config_home().join("autostart")
}

fn xdg_file() -> PathBuf {
    xdg_autostart_dir().join(format!("{APP_ID}.desktop"))
}

/// The stable entry point that the session should execute. A running binary
/// inside an AppImage or a Nix store would disappear after logout/GC.
fn running_binary() -> Option<PathBuf> {
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        let path = PathBuf::from(appimage);
        if path.is_file() {
            return Some(path);
        }
    }
    let current = std::env::current_exe().ok().map(|path| {
        let path = path.canonicalize().unwrap_or(path);
        let text = path.to_string_lossy().to_string();
        PathBuf::from(text.trim_end_matches(" (deleted)").to_string())
    });
    if let Some(path) = &current {
        if path.starts_with("/nix/store") {
            // The profile wrapper (`aura`) on PATH survives store GC.
            return None;
        }
    }
    current.filter(|path| path.is_file()).or_else(|| {
        let home = std::env::var_os("HOME")?;
        let path = PathBuf::from(home).join(".local/bin/aura");
        path.is_file().then_some(path)
    })
}

fn exec_and_try_exec() -> (String, String) {
    if is_flatpak() {
        let id = std::env::var("FLATPAK_ID").unwrap_or_else(|_| APP_ID.to_string());
        return (format!("flatpak run {id} --hidden"), "flatpak".into());
    }
    if is_snap() {
        let name = std::env::var("SNAP_NAME").unwrap_or_else(|_| "aura".into());
        return (format!("snap run {name} --hidden"), "snap".into());
    }
    match running_binary() {
        Some(path) => {
            let text = path.to_string_lossy().to_string();
            (format!("{} --hidden", desktop_exec_quote(&text)), text)
        }
        None => ("aura --hidden".into(), "aura".into()),
    }
}

fn xdg_contents(exec_cmd: &str, try_exec: &str) -> String {
    format!(
        "[Desktop Entry]\n\
        Type=Application\n\
        Name=Aura\n\
        GenericName=Live Wallpaper Manager\n\
        Comment=Animated live wallpaper manager for Linux desktops\n\
        TryExec={try_exec}\n\
        Exec={exec_cmd}\n\
        Icon={APP_ID}\n\
        Terminal=false\n\
        StartupNotify=false\n\
        NoDisplay=true\n\
        Hidden=false\n\
        X-GNOME-Autostart-enabled=true\n\
        X-GNOME-Autostart-Phase=Applications\n\
        X-GNOME-Autostart-Delay=2\n\
        X-Cosmic-Autostart-enabled=true\n\
        X-KDE-autostart-after=panel\n\
        X-KDE-autostart-phase=2\n\
        Categories=Utility;DesktopSettings;\n"
    )
}

fn write_xdg_entry() -> io::Result<()> {
    let dir = xdg_autostart_dir();
    std::fs::create_dir_all(&dir)?;
    let (exec_cmd, try_exec) = exec_and_try_exec();
    let content = xdg_contents(&exec_cmd, &try_exec);
    let path = xdg_file();
    if std::fs::read_to_string(&path).ok().as_deref() != Some(content.as_str()) {
        std::fs::write(path, content)?;
    }
    Ok(())
}

fn remove_xdg_entry() -> io::Result<()> {
    let file = xdg_file();
    if file.exists() {
        std::fs::remove_file(file)?;
    }
    Ok(())
}

/// True when the entry exists and its `Exec` still points at a program that is
/// present. Names without a path separator are resolved by the session PATH.
fn xdg_enabled() -> bool {
    let Ok(content) = std::fs::read_to_string(xdg_file()) else {
        return false;
    };
    if content.lines().any(|line| line.trim() == "Hidden=true") {
        return false;
    }
    let Some(exec) = content
        .lines()
        .find_map(|line| line.strip_prefix("Exec="))
        .map(str::trim)
    else {
        return false;
    };
    let program = exec
        .trim_start_matches('"')
        .split('"')
        .next()
        .unwrap_or(exec)
        .trim();
    !program.contains('/') || Path::new(program).is_file()
}

// ---------------------------------------------------------------------------
// systemd user service fallback
// ---------------------------------------------------------------------------

fn systemd_user_dir() -> PathBuf {
    config_home().join("systemd/user")
}

fn unit_file() -> PathBuf {
    systemd_user_dir().join(UNIT_NAME)
}

fn wants_file() -> PathBuf {
    systemd_user_dir()
        .join("default.target.wants")
        .join(UNIT_NAME)
}

fn launcher_file() -> PathBuf {
    data_home().join("aura/autostart.sh")
}

fn systemd_enabled() -> bool {
    wants_file().exists()
}

fn systemd_quote(value: &str) -> String {
    if !value
        .chars()
        .any(|character| character.is_whitespace() || matches!(character, '"' | '\'' | '\\' | ';'))
    {
        return value.to_string();
    }
    format!(
        "\"{}\"",
        value.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn launcher_contents(binary: Option<&Path>) -> String {
    let exec_line = match binary {
        Some(path) => format!("exec {} --hidden", shell_quote(&path.to_string_lossy())),
        None => "exec aura --hidden".to_string(),
    };
    format!(
        r#"#!/bin/sh
# Generated by Aura. Starts the wallpaper daemon with the graphical session.
# Used only on compositors that do not process XDG autostart entries.
set -u

PATH="$HOME/.local/bin:$HOME/bin:/usr/local/bin:/usr/bin:/bin"
export PATH

if [ -z "${{XDG_RUNTIME_DIR:-}}" ]; then
    XDG_RUNTIME_DIR="/run/user/$(id -u)"
    export XDG_RUNTIME_DIR
fi

# Wait (up to 60 s) for the compositor's Wayland socket.
i=0
while [ "$i" -lt 60 ]; do
    if [ -n "${{WAYLAND_DISPLAY:-}}" ] && [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]; then
        break
    fi
    for socket in "$XDG_RUNTIME_DIR"/wayland-*; do
        [ -S "$socket" ] || continue
        case "$socket" in
            *.lock) continue ;;
        esac
        WAYLAND_DISPLAY="${{socket##*/}}"
        export WAYLAND_DISPLAY
        break 2
    done
    sleep 1
    i=$((i + 1))
done
[ -n "${{WAYLAND_DISPLAY:-}}" ] || exit 1

if [ -z "${{XDG_SESSION_TYPE:-}}" ]; then
    XDG_SESSION_TYPE=wayland
    export XDG_SESSION_TYPE
fi

if [ -z "${{NIRI_SOCKET:-}}" ]; then
    for socket in "$XDG_RUNTIME_DIR"/niri.wayland-*.sock; do
        [ -S "$socket" ] || continue
        NIRI_SOCKET="$socket"
        export NIRI_SOCKET
        break
    done
fi

# Compositors started without a session manager do not export these to the
# systemd user manager. Derive them from the sockets they leave behind.
if [ -z "${{XDG_CURRENT_DESKTOP:-}}" ]; then
    if [ -n "${{NIRI_SOCKET:-}}" ]; then
        XDG_CURRENT_DESKTOP=niri
    elif [ -n "${{SWAYSOCK:-}}" ]; then
        XDG_CURRENT_DESKTOP=sway
    elif [ -n "${{HYPRLAND_INSTANCE_SIGNATURE:-}}" ]; then
        XDG_CURRENT_DESKTOP=Hyprland
    fi
    if [ -n "${{XDG_CURRENT_DESKTOP:-}}" ]; then
        export XDG_CURRENT_DESKTOP
    fi
fi

{exec_line}
"#
    )
}

fn unit_contents(launcher: &Path) -> String {
    format!(
        "[Unit]\n\
        Description=Aura live wallpaper daemon\n\
        Documentation=https://github.com/antwny/aura\n\
        # The launcher waits for the compositor, so this can follow the user\n\
        # session without binding to a graphical-session target.\n\
        StartLimitIntervalSec=0\n\
        \n\
        [Service]\n\
        Type=simple\n\
        ExecStart=/bin/sh {}\n\
        Restart=on-failure\n\
        RestartSec=5\n\
        \n\
        [Install]\n\
        WantedBy=default.target\n",
        systemd_quote(&launcher.to_string_lossy())
    )
}

/// Writes the launcher and unit. Returns whether either file changed, so the
/// caller can skip `systemctl` when nothing needs refreshing.
fn write_systemd_files() -> io::Result<bool> {
    let launcher_path = launcher_file();
    if let Some(parent) = launcher_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let launcher = launcher_contents(running_binary().as_deref());
    let mut changed = false;
    if std::fs::read_to_string(&launcher_path).ok().as_deref() != Some(launcher.as_str()) {
        std::fs::write(&launcher_path, launcher)?;
        changed = true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&launcher_path, std::fs::Permissions::from_mode(0o755));
    }

    let unit_path = unit_file();
    std::fs::create_dir_all(systemd_user_dir())?;
    let unit = unit_contents(&launcher_path);
    if std::fs::read_to_string(&unit_path).ok().as_deref() != Some(unit.as_str()) {
        std::fs::write(&unit_path, unit)?;
        changed = true;
    }
    Ok(changed)
}

fn write_systemd_unit() -> io::Result<()> {
    let changed = write_systemd_files()?;
    if changed || !wants_file().exists() {
        let _ = systemctl(&["daemon-reload"]);
        let _ = systemctl(&["enable", UNIT_NAME]);
    }
    Ok(())
}

fn remove_systemd_unit() {
    let unit = unit_file();
    let launcher = launcher_file();
    let enabled = wants_file().exists();
    if !unit.exists() && !launcher.exists() && !enabled {
        return;
    }
    if enabled && systemd_user_available() {
        let _ = systemctl(&["disable", UNIT_NAME]);
    }
    let _ = std::fs::remove_file(&unit);
    let _ = std::fs::remove_file(&launcher);
    if let Some(parent) = launcher.parent() {
        let _ = std::fs::remove_dir(parent);
    }
    if systemd_user_available() {
        let _ = systemctl(&["daemon-reload"]);
    }
}

/// Quotes a path for the `Exec` key of a desktop entry. Field codes (`%`) are
/// doubled because they would otherwise be expanded by the session.
pub fn desktop_exec_quote(value: &str) -> String {
    let escaped = value
        .replace('%', "%%")
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_exec_quote_escapes_and_quotes() {
        assert_eq!(desktop_exec_quote("/home/user/Aura"), "\"/home/user/Aura\"");
        assert_eq!(
            desktop_exec_quote("/home/user/My Aura/aura"),
            "\"/home/user/My Aura/aura\""
        );
        assert_eq!(
            desktop_exec_quote("/tmp/a\"b$c`d\\e"),
            "\"/tmp/a\\\"b\\$c\\`d\\\\e\""
        );
        assert_eq!(desktop_exec_quote("/tmp/100%"), "\"/tmp/100%%\"");
    }

    #[test]
    fn xdg_entry_has_required_keys() {
        let content = xdg_contents("\"/opt/aura\" --hidden", "/opt/aura");
        assert!(content.contains("Type=Application"));
        assert!(content.contains("Exec=\"/opt/aura\" --hidden"));
        assert!(content.contains("TryExec=/opt/aura"));
        assert!(content.contains("X-Cosmic-Autostart-enabled=true"));
        assert!(content.contains("X-GNOME-Autostart-enabled=true"));
        assert!(content.contains("X-KDE-autostart-after=panel"));
    }

    #[test]
    fn launcher_waits_for_compositor_and_exports_session() {
        let script = launcher_contents(Some(Path::new("/home/user/.local/bin/aura")));
        assert!(script.starts_with("#!/bin/sh"));
        assert!(script.contains("wayland-*"));
        assert!(script.contains("NIRI_SOCKET"));
        assert!(script.contains("XDG_CURRENT_DESKTOP=niri"));
        assert!(script.contains("exec '/home/user/.local/bin/aura' --hidden"));
    }

    #[test]
    fn launcher_quotes_paths_with_spaces() {
        let script = launcher_contents(Some(Path::new("/home/user/My Aura/aura")));
        assert!(script.contains("exec '/home/user/My Aura/aura' --hidden"));
    }

    #[test]
    fn unit_points_at_launcher() {
        let unit = unit_contents(Path::new("/home/user/.local/share/aura/autostart.sh"));
        assert!(unit.contains("ExecStart=/bin/sh /home/user/.local/share/aura/autostart.sh"));
        assert!(unit.contains("WantedBy=default.target"));
        assert!(!unit.contains("graphical-session.target"));
    }

    #[test]
    fn xdg_entry_round_trips_on_disk() {
        write_xdg_entry().expect("write xdg entry");
        assert!(xdg_file().is_file());
        assert!(xdg_enabled());
        let content = std::fs::read_to_string(xdg_file()).unwrap();
        assert!(content.contains("Exec="));
        remove_xdg_entry().expect("remove xdg entry");
        assert!(!xdg_enabled());
    }

    #[test]
    fn systemd_unit_round_trips_on_disk() {
        let changed = write_systemd_files().expect("write systemd files");
        assert!(changed);
        let launcher = launcher_file();
        let unit = unit_file();
        assert!(launcher.is_file());
        assert!(unit.is_file());
        let unit_text = std::fs::read_to_string(&unit).unwrap();
        assert!(unit_text.contains(&launcher.to_string_lossy().to_string()));
        assert!(std::fs::read_to_string(&launcher)
            .unwrap()
            .contains("--hidden"));
        let _ = std::fs::remove_file(&unit);
        let _ = std::fs::remove_file(&launcher);
    }
}
