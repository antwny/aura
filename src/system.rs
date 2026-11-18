use zbus::Connection;

/// PIDs currently registered with the Feral GameMode session service.
/// None when GameMode is not reachable on the session bus.
async fn gamemode_registered_pids() -> Option<Vec<u32>> {
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
    let lower = cmdline.to_lowercase().replace('\0', " ");
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

fn is_zombie_status(status: &str) -> bool {
    status.lines().any(|line| {
        line.strip_prefix("State:")
            .and_then(|rest| rest.split_whitespace().next())
            .is_some_and(|state| state == "Z")
    })
}

fn is_zombie_pid(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/status"))
        .is_ok_and(|status| is_zombie_status(&status))
}

fn heavy_process_pids() -> Vec<u32> {
    let mut pids = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.filter_map(|e| e.ok()) {
            let pid: u32 = match entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse().ok())
            {
                Some(pid) => pid,
                None => continue,
            };
            if is_zombie_pid(pid) {
                continue;
            }
            if std::fs::read_to_string(entry.path().join("cmdline"))
                .map_or(false, |cmdline| is_heavy_process_cmdline(&cmdline))
            {
                pids.push(pid);
            }
        }
    }
    pids
}

fn exe_tokens(cmdline: &[u8]) -> Vec<String> {
    let mut tokens = Vec::new();
    for arg in cmdline.split(|byte| *byte == 0) {
        let base = String::from_utf8_lossy(arg)
            .rsplit(|c| c == '/' || c == '\\')
            .next()
            .unwrap_or_default()
            .to_lowercase();
        if base.ends_with(".exe") && base.len() > 4 {
            tokens.push(base.clone());
            tokens.push(base.trim_end_matches(".exe").to_owned());
        }
    }
    tokens
}

fn game_window_tokens(pids: &[u32]) -> Vec<String> {
    let mut tokens = Vec::new();
    for pid in pids {
        if let Ok(comm) = std::fs::read_to_string(format!("/proc/{pid}/comm")) {
            let comm = comm.trim().to_lowercase();
            if !comm.is_empty() {
                tokens.push(comm);
            }
        }
        if let Ok(raw) = std::fs::read(format!("/proc/{pid}/cmdline")) {
            tokens.extend(exe_tokens(&raw));
        }
    }
    tokens
}

fn window_matches_game(tokens: &[String], app_id: &str, title: &str) -> bool {
    let app_id = app_id.to_lowercase();
    let title = title.to_lowercase();
    tokens.iter().any(|token| {
        !token.is_empty() && (app_id.contains(token) || title.contains(token))
    })
}

fn niri_json_output(args: &[&str]) -> Option<serde_json::Value> {
    let output = std::process::Command::new("niri").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

fn active_workspace_ids() -> Option<Vec<i64>> {
    let workspaces = niri_json_output(&["msg", "--json", "workspaces"])?;
    let ids: Vec<i64> = workspaces
        .as_array()?
        .iter()
        .filter_map(|workspace| {
            workspace
                .get("is_active")
                .and_then(|active| active.as_bool())
                .filter(|active| *active)?;
            workspace.get("id").and_then(|id| id.as_i64())
        })
        .collect();
    Some(ids)
}

fn game_window_visible(tokens: &[String]) -> bool {
    let windows = match niri_json_output(&["msg", "--json", "windows"]) {
        Some(windows) => windows,
        None => return false,
    };
    let Some(windows) = windows.as_array() else {
        return false;
    };
    let active = active_workspace_ids().filter(|ids| !ids.is_empty());
    windows
        .iter()
        .filter(|window| match (
            &active,
            window.get("workspace_id").and_then(|id| id.as_i64()),
        ) {
            (Some(ids), Some(workspace)) => ids.contains(&workspace),
            _ => true,
        })
        .any(|window| {
            let app_id = window
                .get("app_id")
                .and_then(|id| id.as_str())
                .unwrap_or_default();
            let title = window
                .get("title")
                .and_then(|title| title.as_str())
                .unwrap_or_default();
            window_matches_game(tokens, app_id, title)
        })
}

pub async fn check_if_fullscreen_game_active() -> bool {
    let mut pids = gamemode_registered_pids().await.unwrap_or_default();
    let proc_pids = tokio::task::spawn_blocking(heavy_process_pids)
        .await
        .unwrap_or_default();
    pids.extend(proc_pids);
    if pids.is_empty() {
        return false;
    }
    if std::env::var_os("NIRI_SOCKET").is_none() {
        return true;
    }
    let tokens = tokio::task::spawn_blocking(move || game_window_tokens(&pids))
        .await
        .unwrap_or_default();
    tokio::task::spawn_blocking(move || game_window_visible(&tokens))
        .await
        .unwrap_or(false)
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
        assert!(is_heavy_process_cmdline(
            "/opt/heroic/heroic\0--wine\0C:\\Games\\Overwatch\\Overwatch.exe"
        ));
        assert!(is_heavy_process_cmdline("steam_app_12345"));
        assert!(is_heavy_process_cmdline("wine64-preloader app.exe"));
        assert!(is_heavy_process_cmdline("lutris-wrapper game"));
        assert!(!is_heavy_process_cmdline("/usr/bin/bash"));
        assert!(!is_heavy_process_cmdline("cosmic-panel"));
        assert!(!is_heavy_process_cmdline("/usr/bin/protonvpn-app"));
        assert!(!is_heavy_process_cmdline("/opt/heroic/heroic"));
        assert!(!is_heavy_process_cmdline("heroic\0game\0--wine"));
        assert!(!is_heavy_process_cmdline(""));
    }

    #[test]
    fn test_is_zombie_status() {
        assert!(is_zombie_status("Name:\tsober\nState:\tZ (zombie)\n"));
        assert!(!is_zombie_status("Name:\tsober\nState:\tS (sleeping)\n"));
        assert!(!is_zombie_status("Name:\tmpvpaper\nState:\tR (running)\n"));
        assert!(!is_zombie_status(""));
    }

    #[test]
    fn test_exe_tokens() {
        let raw = b"/home/u/.wine/drive_c/Games/Overwatch/Overwatch.exe\0--arg\0";
        assert_eq!(exe_tokens(raw), vec!["overwatch.exe", "overwatch"]);
        assert!(exe_tokens(b"/usr/bin/gamescope\0-W\01920\0").is_empty());
        assert!(exe_tokens(b"").is_empty());
    }

    #[test]
    fn test_window_matches_game() {
        let overwatch = vec!["overwatch.exe".to_string(), "overwatch".to_string()];
        assert!(window_matches_game(&overwatch, "steam_app_12345", "Overwatch"));
        assert!(window_matches_game(&overwatch, "Overwatch.exe", ""));
        let sober = vec!["sober".to_string()];
        assert!(window_matches_game(&sober, "org.vinegarhq.Sober", "Sober"));
        assert!(!window_matches_game(&overwatch, "kitty", "OC | Analisis"));
        assert!(!window_matches_game(&[], "org.vinegarhq.Sober", "Sober"));
        assert!(!window_matches_game(&["".to_string()], "kitty", "kitty"));
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
