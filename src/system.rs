use zbus::Connection;

/// Queries registered game processes from the Feral GameMode session service.
/// Returns registered game PIDs, or `None` when GameMode is unavailable.
pub async fn check_gamemode() -> Option<Vec<u32>> {
    let conn = tokio::time::timeout(
        std::time::Duration::from_millis(300),
        Connection::session(),
    )
    .await
    .ok()?
    .ok()?;

    let proxy = zbus::Proxy::new(
        &conn,
        "com.feralinteractive.GameMode",
        "/com/feralinteractive/GameMode",
        "com.feralinteractive.GameMode",
    )
    .await
    .ok()?;

    let games: Vec<(i32, zbus::zvariant::OwnedObjectPath)> = tokio::time::timeout(
        std::time::Duration::from_millis(300),
        proxy.call("ListGames", &()),
    )
    .await
    .ok()?
    .ok()?;

    Some(
        games
            .into_iter()
            .filter_map(|(pid, _)| u32::try_from(pid).ok())
            .collect(),
    )
}

/// Queries UPower over system D-Bus to determine if device is running on battery.
/// Returns Some(true) if on battery, Some(false) if on AC power,
/// or None if UPower is not reachable.
pub async fn check_on_battery() -> Option<bool> {
    let conn = tokio::time::timeout(
        std::time::Duration::from_millis(300),
        Connection::system(),
    )
    .await
    .ok()?
    .ok()?;

    let proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower",
        "org.freedesktop.UPower",
    )
    .await
    .ok()?;

    let on_battery: bool = tokio::time::timeout(
        std::time::Duration::from_millis(300),
        proxy.get_property("OnBattery"),
    )
    .await
    .ok()?
    .ok()?;

    Some(on_battery)
}

pub fn is_heavy_process_cmdline(cmdline: &str) -> bool {
    let lower = cmdline.replace('\0', " ").to_lowercase();
    if lower.contains("protonvpn") {
        return false;
    }
    if lower.contains("gamescope")
        || lower.contains("proton")
        || lower.contains("steam_app")
        || lower.contains("heroic --wine")
        || lower.contains("wine64-preloader")
        || lower.contains("lutris-wrapper")
    {
        return true;
    }
    false
}

fn process_is_zombie(stat: &str) -> bool {
    stat.rsplit_once(')')
        .map(|(_, tail)| tail.trim_start().starts_with('Z'))
        .unwrap_or(false)
}

pub async fn check_if_fullscreen_game_active() -> bool {
    let game_pids = tokio::task::spawn_blocking(scan_game_processes)
        .await
        .unwrap_or_default();
    let mut game_pids = game_pids;
    // GameMode's ListGames response includes each registered game PID.
    if let Some(pids) = check_gamemode().await {
        game_pids.extend(pids);
    }
    game_pids.sort_unstable();
    game_pids.dedup();
    if game_pids.is_empty() {
        return false;
    }
    if crate::engine::DesktopEnvironment::detect() == crate::engine::DesktopEnvironment::Niri {
        return tokio::task::spawn_blocking(move || niri_has_visible_game(&game_pids))
            .await
            .unwrap_or(false);
    }
    true
}

fn scan_game_processes() -> Vec<u32> {
    let mut pids = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else { return pids };
    for entry in entries.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else { continue };
        let path = entry.path();
        if std::fs::read_to_string(path.join("stat"))
            .ok().is_some_and(|stat| process_is_zombie(&stat)) { continue; }
        let cmdline = std::fs::read(path.join("cmdline")).unwrap_or_default();
        let exe = std::fs::read_link(path.join("exe")).ok();
        let text = String::from_utf8_lossy(&cmdline);
        let exe_name = exe.as_ref().and_then(|p| p.file_name()).map(|s| s.to_string_lossy());
        if is_heavy_process_cmdline(&text) || exe_name.as_deref().is_some_and(is_heavy_process_name) {
            pids.push(pid);
        }
    }
    pids
}

fn is_heavy_process_name(name: &str) -> bool {
    let name = name.to_lowercase();
    if name.contains("protonvpn") { return false; }
    if name.ends_with(".exe") { return true; }
    ["gamescope", "proton", "wine", "steam_app", "lutris-wrapper", "reaper", "srt-bwrap"]
        .iter().any(|needle| name.contains(needle))
}

fn niri_has_visible_game(pids: &[u32]) -> bool {
    let Ok(workspaces_output) = std::process::Command::new("niri").args(["msg", "--json", "workspaces"]).output() else { return false };
    if !workspaces_output.status.success() { return false; }
    let Ok(workspaces) = serde_json::from_slice::<serde_json::Value>(&workspaces_output.stdout) else { return false };
    let active: Vec<u64> = workspaces.as_array().into_iter().flatten()
        .filter(|workspace| workspace.get("is_active").and_then(|v| v.as_bool()).unwrap_or(false))
        .filter_map(|workspace| workspace.get("id").and_then(|v| v.as_u64())).collect();
    if active.is_empty() { return false; }
    let Ok(output) = std::process::Command::new("niri").args(["msg", "--json", "windows"]).output() else { return false };
    if !output.status.success() { return false; }
    let Ok(windows) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else { return false };
    let visible: Vec<_> = windows.as_array().into_iter().flatten().filter(|window| {
        window.get("workspace_id").and_then(|v| v.as_u64()).is_some_and(|id| active.contains(&id))
    }).collect();
    visible.iter().any(|window| {
        if window.get("pid").and_then(|v| v.as_u64()).is_some_and(|pid| pids.contains(&(pid as u32))) { return true; }
        let labels = ["app_id", "title"].iter().filter_map(|key| window.get(key).and_then(|v| v.as_str())).collect::<Vec<_>>();
        pids.iter().any(|pid| process_tokens(*pid).iter().any(|token| labels.iter().any(|label| label.to_lowercase().contains(token))))
    })
}

fn process_tokens(pid: u32) -> Vec<String> {
    let path = std::path::PathBuf::from(format!("/proc/{pid}"));
    let mut tokens = Vec::new();
    if let Ok(exe) = std::fs::read_link(path.join("exe")) { if let Some(name) = exe.file_name() { tokens.push(name.to_string_lossy().to_lowercase()); } }
    if let Ok(comm) = std::fs::read_to_string(path.join("comm")) { tokens.push(comm.trim().to_lowercase()); }
    if let Ok(cmdline) = std::fs::read(path.join("cmdline")) {
        for arg in cmdline.split(|byte| *byte == 0).filter(|arg| !arg.is_empty()) {
            let arg = String::from_utf8_lossy(arg);
            let candidate = std::path::Path::new(arg.as_ref())
                .file_name()
                .map(|name| name.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if candidate.contains(".exe") || is_heavy_process_name(&candidate) {
                tokens.push(candidate);
            }
        }
    }
    tokens.retain(|token| !token.is_empty());
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_heavy_process_cmdline() {
        assert!(is_heavy_process_cmdline("/usr/bin/gamescope -W 1920"));
        assert!(is_heavy_process_cmdline("proton run game.exe"));
        assert!(is_heavy_process_cmdline("heroic --wine game.exe"));
        assert!(is_heavy_process_cmdline("heroic\0--wine\0game.exe"));
        assert!(is_heavy_process_cmdline("steam_app_12345"));
        assert!(is_heavy_process_cmdline("wine64-preloader app.exe"));
        assert!(is_heavy_process_cmdline("lutris-wrapper game"));
        assert!(!is_heavy_process_cmdline("/usr/bin/bash"));
        assert!(!is_heavy_process_cmdline("cosmic-panel"));
        assert!(!is_heavy_process_cmdline("/usr/bin/protonvpn-app"));
        assert!(!is_heavy_process_cmdline("/opt/heroic/heroic"));
        assert!(!is_heavy_process_cmdline(""));
    }

    #[test]
    fn process_stat_detects_zombies_without_confusing_names_with_spaces() {
        assert!(process_is_zombie("42 (game process) Z 1 2 3"));
        assert!(!process_is_zombie("42 (game process) S 1 2 3"));
    }

    #[test]
    fn process_executable_names_are_detected() {
        assert!(is_heavy_process_name("Overwatch.exe"));
        assert!(is_heavy_process_name("srt-bwrap"));
        assert!(!is_heavy_process_name("protonvpn-app"));
    }
}

/// Best-effort session lock detection through logind's LockedHint. Returns
/// false when the desktop does not publish the hint, so it never pauses on a
/// false positive.
pub async fn check_session_locked() -> bool {
    tokio::task::spawn_blocking(|| {
        let session = std::env::var("XDG_SESSION_ID").unwrap_or_default();
        let mut command = std::process::Command::new("loginctl");
        if session.is_empty() {
            command.args(["show-session", "self", "-p", "LockedHint", "--value"]);
        } else {
            command.args(["show-session", &session, "-p", "LockedHint", "--value"]);
        }
        let Ok(output) = command.output() else {
            return false;
        };
        output.status.success()
            && String::from_utf8_lossy(&output.stdout).trim().eq_ignore_ascii_case("yes")
    })
    .await
    .unwrap_or(false)
}
