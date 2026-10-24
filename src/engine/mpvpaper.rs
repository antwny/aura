use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

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

pub fn socket_dir() -> PathBuf {
    #[cfg(test)]
    crate::test_support::init();

    if let Some(xdg) = std::env::var_os("XDG_RUNTIME_DIR") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("aura");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".cache/aura/ipc")
}

pub fn shader_cache_dir() -> PathBuf {
    #[cfg(test)]
    crate::test_support::init();

    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            let p = PathBuf::from(xdg).join("aura/mpv_shaders");
            let _ = std::fs::create_dir_all(&p);
            return p;
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let p = PathBuf::from(home).join(".cache/aura/mpv_shaders");
    let _ = std::fs::create_dir_all(&p);
    p
}

pub fn socket_path_for_output(output: &str) -> PathBuf {
    let clean: String = output.chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
    let name = if clean.is_empty() { "all".to_string() } else { clean };
    let dir = socket_dir();
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("mpv_{}.sock", name))
}

fn output_from_socket_stem(stem: &str) -> String {
    match stem.strip_prefix("mpv_").unwrap_or(stem) {
        // The wildcard output is stored on disk as mpv_all.sock.
        "all" => "*".to_string(),
        output => output.to_string(),
    }
}

pub fn send_ipc_command(sock_path: &Path, json_cmd: &str) -> Result<String, String> {
    let mut stream = UnixStream::connect(sock_path)
        .map_err(|e| format!("IPC connect error: {}", e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_millis(300)))
        .map_err(|e| format!("IPC timeout error: {}", e))?;
    stream.set_write_timeout(Some(std::time::Duration::from_millis(300)))
        .map_err(|e| format!("IPC timeout error: {}", e))?;

    stream.write_all(json_cmd.as_bytes())
        .map_err(|e| format!("IPC write error: {}", e))?;
    stream.write_all(b"\n")
        .map_err(|e| format!("IPC write newline error: {}", e))?;
    stream.flush()
        .map_err(|e| format!("IPC flush error: {}", e))?;

    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf)
        .map_err(|e| format!("IPC read error: {}", e))?;
    Ok(String::from_utf8_lossy(&buf[..n]).to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcStatus {
    Alive,
    Dead,
    Unknown,
}

pub fn ipc_status(sock_path: &Path) -> IpcStatus {
    if !sock_path.exists() {
        return IpcStatus::Dead;
    }

    let mut stream = match UnixStream::connect(sock_path) {
        Ok(stream) => stream,
        Err(e) => {
            if matches!(
                e.kind(),
                ErrorKind::NotFound
                    | ErrorKind::ConnectionRefused
                    | ErrorKind::ConnectionReset
                    | ErrorKind::ConnectionAborted
            ) {
                let _ = std::fs::remove_file(sock_path);
                return IpcStatus::Dead;
            }
            return IpcStatus::Unknown;
        }
    };

    let timeout = Some(Duration::from_millis(400));
    if stream.set_read_timeout(timeout).is_err() || stream.set_write_timeout(timeout).is_err() {
        return IpcStatus::Unknown;
    }
    if stream
        .write_all(b"{\"command\": [\"get_property\", \"pause\"]}\n")
        .is_err()
    {
        return IpcStatus::Unknown;
    }

    let mut buf = [0u8; 4096];
    match stream.read(&mut buf) {
        Ok(0) => IpcStatus::Unknown,
        Ok(n) => {
            let response = String::from_utf8_lossy(&buf[..n]);
            if response.contains(r#""error":"success""#) || response.contains("success") {
                IpcStatus::Alive
            } else {
                IpcStatus::Unknown
            }
        }
        Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
            IpcStatus::Unknown
        }
        Err(_) => IpcStatus::Unknown,
    }
}

pub fn ipc_is_alive(sock_path: &Path) -> bool {
    matches!(ipc_status(sock_path), IpcStatus::Alive)
}

pub fn ipc_loadfile(sock_path: &Path, file_path: &str) -> Result<(), String> {
    let cmd = serde_json::json!({
        "command": ["loadfile", file_path, "replace"]
    });
    let cmd_str = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
    let resp = send_ipc_command(sock_path, &cmd_str)?;
    if resp.contains(r#""error":"success""#) {
        Ok(())
    } else {
        Err(format!("IPC loadfile failed: {}", resp))
    }
}

pub fn ipc_set_property<T: serde::Serialize>(sock_path: &Path, prop: &str, val: T) -> Result<(), String> {
    let cmd = serde_json::json!({
        "command": ["set_property", prop, val]
    });
    let cmd_str = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
    let resp = send_ipc_command(sock_path, &cmd_str)?;
    if resp.contains(r#""error":"success""#) {
        Ok(())
    } else {
        Err(format!("IPC set_property failed: {}", resp))
    }
}

/// Reads a property from mpv's JSON IPC and returns its `data` value.
pub fn ipc_get_value(sock_path: &Path, prop: &str) -> Option<serde_json::Value> {
    let cmd = serde_json::json!({
        "command": ["get_property", prop]
    });
    let cmd_str = serde_json::to_string(&cmd).ok()?;
    let response = send_ipc_command(sock_path, &cmd_str).ok()?;
    // mpv may batch several JSON objects in one read. Keep the first complete line.
    let line = response.lines().find(|line| !line.trim().is_empty())?;
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("error").and_then(|v| v.as_str()) != Some("success") {
        return None;
    }
    value.get("data").cloned()
}

/// Waits until a freshly loaded file has a configured video output and has had
/// time to present at least a couple of frames. This is the readiness barrier
/// used by the transactional wallpaper transition.
pub fn ipc_wait_first_frame(sock_path: &Path, expected_path: Option<&Path>, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    let mut configured_since = None;

    while Instant::now() < deadline {
        if !ipc_is_alive(sock_path) {
            return false;
        }

        let configured = ipc_get_value(sock_path, "vo-configured")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let has_size = ipc_get_value(sock_path, "width")
            .and_then(|value| value.as_i64())
            .is_some_and(|width| width > 0)
            || ipc_get_value(sock_path, "video-params")
                .and_then(|value| value.get("w").and_then(|w| w.as_i64()))
                .is_some_and(|width| width > 0);
        // Reusing a live IPC socket means vo-configured and width are still true
        // from the previous file. Require mpv to actually report the new path
        // before accepting the readiness barrier.
        let loaded_path_matches = expected_path.is_none_or(|expected| {
            ipc_get_value(sock_path, "path")
                .and_then(|value| value.as_str().map(str::to_owned))
                .is_some_and(|actual| {
                    let actual = Path::new(&actual);
                    actual == expected
                        || (actual.file_name().is_some()
                            && actual.file_name() == expected.file_name())
                })
        });

        if configured && has_size && loaded_path_matches {
            let since = configured_since.get_or_insert_with(Instant::now);
            // Five frames at 60 Hz plus slack for slow image decoders.
            if since.elapsed() >= Duration::from_millis(80) {
                return true;
            }
        } else {
            configured_since = None;
        }

        std::thread::sleep(Duration::from_millis(15));
    }

    false
}

pub fn ipc_get_pause(sock_path: &Path) -> Option<bool> {
    let get_cmd = r#"{"command": ["get_property", "pause"]}"#;
    if let Ok(resp) = send_ipc_command(sock_path, get_cmd) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&resp) {
            return val.get("data").and_then(|d| d.as_bool());
        }
    }
    None
}

pub fn ipc_get_pid(sock_path: &Path) -> Option<u32> {
    let get_cmd = r#"{"command": ["get_property", "pid"]}"#;
    if let Ok(resp) = send_ipc_command(sock_path, get_cmd) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&resp) {
            return val.get("data").and_then(|d| d.as_u64()).map(|p| p as u32);
        }
    }
    None
}

pub fn can_use_systemd_run() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        if std::path::Path::new("/.flatpak-info").exists()
            || std::env::var_os("FLATPAK_ID").is_some()
            || std::env::var_os("SNAP").is_some()
        {
            return false;
        }
        if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none()
            && std::env::var_os("XDG_RUNTIME_DIR").is_none()
        {
            return false;
        }
        Command::new("systemd-run")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

pub fn is_pid_stopped(pid: u32) -> bool {
    let status_path = format!("/proc/{}/status", pid);
    if let Ok(content) = std::fs::read_to_string(&status_path) {
        for line in content.lines() {
            if line.starts_with("State:") {
                return line.contains("T (stopped)") || line.contains("\tT");
            }
        }
    }
    false
}

pub fn is_mpvpaper_pid(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{}/comm", pid))
        .map(|comm| comm.trim() == "mpvpaper")
        .unwrap_or(false)
}

fn pid_for_socket(sock_path: &Path) -> Option<u32> {
    let expected = format!("--input-ipc-server={}", sock_path.display());
    let entries = std::fs::read_dir("/proc").ok()?;
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        if !is_mpvpaper_pid(pid) {
            continue;
        }
        let Ok(cmdline) = std::fs::read(format!("/proc/{}/cmdline", pid)) else {
            continue;
        };
        if String::from_utf8_lossy(&cmdline).contains(&expected) {
            return Some(pid);
        }
    }
    None
}

pub fn find_mpvpaper_pids() -> Vec<u32> {
    let mut pids = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name();
            let s = name.to_string_lossy();
            if s.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(pid) = s.parse::<u32>() {
                    let comm_path = format!("/proc/{}/comm", pid);
                    if let Ok(comm) = std::fs::read_to_string(&comm_path) {
                        if comm.trim() == "mpvpaper" {
                            pids.push(pid);
                        }
                    }
                }
            }
        }
    }
    pids
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineHealth {
    Ready,
    MissingBinary,
    MissingLibrary(String),
    ExecutionError(String),
}

pub fn detect_distro_command() -> (&'static str, &'static str) {
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        let content_lower = content.to_lowercase();
        if content_lower.contains("cachyos")
            || content_lower.contains("arch")
            || content_lower.contains("manjaro")
            || content_lower.contains("endeavouros")
        {
            return ("Arch / CachyOS", "sudo pacman -S --needed mpv ffmpeg");
        } else if content_lower.contains("pop")
            || content_lower.contains("ubuntu")
            || content_lower.contains("debian")
            || content_lower.contains("mint")
        {
            return ("Pop!_OS / Ubuntu / Debian", "sudo apt install -y libmpv2 ffmpeg");
        } else if content_lower.contains("fedora") || content_lower.contains("nobara") {
            return ("Fedora", "sudo dnf install -y mpv-libs ffmpeg-free");
        } else if content_lower.contains("suse") {
            return ("openSUSE", "sudo zypper install -y mpv ffmpeg");
        } else if content_lower.contains("void") {
            return ("Void Linux", "sudo xbps-install -S mpv ffmpeg");
        }
    }
    ("Linux", "sudo apt install -y libmpv2 ffmpeg || sudo pacman -S --needed mpv ffmpeg")
}

pub fn detect_os_pretty_name() -> String {
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("PRETTY_NAME=") {
                let name = rest.trim_matches('"').trim_matches('\'').trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("NAME=") {
                let name = rest.trim_matches('"').trim_matches('\'').trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
    }
    "Linux".to_string()
}

pub fn check_engine_health() -> EngineHealth {
    let bin = resolve_mpvpaper_binary();
    match Command::new(&bin).arg("-h").stdout(Stdio::null()).stderr(Stdio::piped()).output() {
        Ok(output) => {
            if output.status.success() {
                EngineHealth::Ready
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                if output.status.code() == Some(127) || stderr.contains("libmpv") {
                    EngineHealth::MissingLibrary("libmpv".to_string())
                } else {
                    EngineHealth::ExecutionError(stderr)
                }
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                EngineHealth::MissingBinary
            } else {
                EngineHealth::ExecutionError(e.to_string())
            }
        }
    }
}

pub struct WallpaperEngine {
    // Stores active process handle per output
    processes: HashMap<String, Child>,
    // Stores active IPC socket path per output
    sockets: HashMap<String, PathBuf>,
    // Stores whether active wallpaper on output is an image
    output_is_image: HashMap<String, bool>,
    // Stores the last successfully applied path per output for rollback
    current_paths: HashMap<String, String>,
    pub is_paused: bool,
}

impl WallpaperEngine {
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
            sockets: HashMap::new(),
            output_is_image: HashMap::new(),
            current_paths: HashMap::new(),
            is_paused: false,
        }
    }

    fn own_pids(&self) -> Vec<u32> {
        let mut pids: Vec<u32> = self
            .sockets
            .values()
            .filter_map(|sock| {
                pid_for_socket(sock).or_else(|| ipc_get_pid(sock).filter(|pid| is_mpvpaper_pid(*pid)))
            })
            .collect();
        for child in self.processes.values() {
            pids.push(child.id());
        }
        pids.sort_unstable();
        pids.dedup();
        pids.retain(|pid| is_mpvpaper_pid(*pid));
        pids
    }

    fn previous_wallpapers(&self, output: &str) -> Vec<(String, String)> {
        let mut previous = Vec::new();
        if output == "*" {
            if let Some(path) = self.current_paths.get("*") {
                previous.push(("*".to_string(), path.clone()));
            }
            for (out, path) in &self.current_paths {
                if out != "*" {
                    previous.push((out.clone(), path.clone()));
                }
            }
        } else if let Some(path) = self.current_paths.get(output) {
            // A specific output transition must never restore over the wildcard:
            // the wildcard is a separate, still-running underlay.
            previous.push((output.to_string(), path.clone()));
        }
        previous
    }

    fn restore_previous(
        &mut self,
        previous: &[(String, String)],
        scaling: &str,
        mute: bool,
        volume: u8,
        hwdec: &str,
        auto_pause: bool,
        gpu_pref: &str,
    ) {
        for (output, path) in previous {
            let _ = self.set_wallpaper_inner(
                output, path, scaling, mute, volume, hwdec, auto_pause, gpu_pref, false, false,
                true,
            );
        }
    }

    pub fn processes_keys(&self) -> Vec<String> {
        self.processes.keys().cloned().collect()
    }

    /// Outputs with either a process handle or a discovered IPC socket.
    pub fn active_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .processes
            .keys()
            .chain(self.sockets.keys())
            .cloned()
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    pub fn has_output_player(&mut self, output: &str) -> bool {
        self.ensure_sockets_discovered();
        self.sockets.contains_key(output) || self.processes.contains_key(output)
    }

    pub fn has_wildcard_player(&mut self) -> bool {
        self.ensure_sockets_discovered();
        self.sockets.contains_key("*") || self.processes.contains_key("*")
    }

    pub fn current_path(&self, output: &str) -> Option<String> {
        self.current_paths.get(output).cloned()
    }

    pub fn is_output_running(&mut self, output: &str) -> bool {
        if crate::engine::gnome::is_active() {
            return false;
        }
        let sock = self
            .sockets
            .get(output)
            .cloned()
            .unwrap_or_else(|| socket_path_for_output(output));
        if ipc_is_alive(&sock) {
            return true;
        }
        self.processes
            .get_mut(output)
            .is_some_and(|child| matches!(child.try_wait(), Ok(None)))
    }

    pub fn reap_dead_processes(&mut self) -> Vec<String> {
        let mut dead = Vec::new();
        let sockets = self.sockets.clone();
        self.processes.retain(|output, child| match child.try_wait() {
            Ok(Some(_status)) => {
                let ipc_alive = sockets
                    .get(output)
                    .map(|sock| ipc_is_alive(sock))
                    .unwrap_or(false);
                if ipc_alive {
                    true
                } else {
                    dead.push(output.clone());
                    false
                }
            }
            Ok(None) => true,
            Err(_) => {
                dead.push(output.clone());
                false
            }
        });
        for out in &dead {
            if let Some(sock) = self.sockets.remove(out) {
                if !ipc_is_alive(&sock) {
                    let _ = std::fs::remove_file(sock);
                }
            }
            self.output_is_image.remove(out);
            self.current_paths.remove(out);
        }
        dead
    }

    pub fn ensure_sockets_discovered(&mut self) {
        let sock_dir = socket_dir();
        let Ok(entries) = std::fs::read_dir(&sock_dir) else {
            return;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("sock") {
                continue;
            }
            // In-flight transition sockets are private to the worker and must
            // not be mistaken for a real output.
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.contains(".next"))
            {
                continue;
            }
            let status = ipc_status(&path);
            if matches!(status, IpcStatus::Alive | IpcStatus::Unknown) {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                self.sockets.insert(output_from_socket_stem(name), path);
            }
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
        auto_pause: bool,
        gpu_pref: &str,
    ) -> Result<u32, std::io::Error> {
        // Reuse the existing mpv surface when it is alive so the wallpaper
        // changes in place without a compositor-visible black frame.
        self.set_wallpaper_inner(
            output, video_path, scaling, mute, volume, hwdec, auto_pause, gpu_pref, true, false,
            false,
        )
    }

    /// Recreates the wildcard surface without touching per-output overrides.
    /// Used for hotplug: mpvpaper binds the monitor list when it starts, so IPC
    /// alone cannot attach a newly connected output.
    #[allow(clippy::too_many_arguments)]
    pub fn restart_wildcard(
        &mut self,
        video_path: &str,
        scaling: &str,
        mute: bool,
        volume: u8,
        hwdec: &str,
        auto_pause: bool,
        gpu_pref: &str,
    ) -> Result<u32, std::io::Error> {
        self.set_wallpaper_inner(
            "*", video_path, scaling, mute, volume, hwdec, auto_pause, gpu_pref, false, true,
            true,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn set_wallpaper_inner(
        &mut self,
        output: &str,
        video_path: &str,
        scaling: &str,
        mute: bool,
        volume: u8,
        hwdec: &str,
        auto_pause: bool,
        gpu_pref: &str,
        allow_restore: bool,
        force_spawn: bool,
        preserve_overrides: bool,
    ) -> Result<u32, std::io::Error> {
        if crate::engine::gnome::is_active() {
            let result = self.set_wallpaper_gnome(video_path, scaling, mute, volume);
            if result.is_ok() {
                self.current_paths.clear();
                self.current_paths.insert("*".to_string(), video_path.to_string());
            }
            return result;
        }

        let is_remote = video_path.starts_with("http://")
            || video_path.starts_with("https://")
            || video_path.starts_with("rtsp://");
        if !is_remote && !Path::new(video_path).is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("El archivo de fondo no existe: {}", video_path),
            ));
        }

        let previous = if allow_restore {
            self.previous_wallpapers(output)
        } else {
            Vec::new()
        };
        let retire_overrides = output == "*" && !preserve_overrides;
        let result = self.apply_wallpaper(
            output,
            video_path,
            scaling,
            mute,
            volume,
            hwdec,
            auto_pause,
            gpu_pref,
            force_spawn,
            retire_overrides,
        );

        match result {
            Ok(pid) => Ok(pid),
            Err(e) => {
                // The transactional transition keeps the old surface alive until
                // the new one is ready. Only restore if a previous surface is no
                // longer running; otherwise there is nothing to recover.
                if allow_restore && !previous.is_empty() {
                    let missing: Vec<_> = previous
                        .iter()
                        .filter(|(out, _)| !self.is_output_running(out))
                        .cloned()
                        .collect();
                    if !missing.is_empty() {
                        eprintln!(
                            "[Aura Engine] No se pudo aplicar '{}', restaurando el fondo anterior.",
                            video_path
                        );
                        self.restore_previous(&missing, scaling, mute, volume, hwdec, auto_pause, gpu_pref);
                    }
                }
                Err(e)
            }
        }
    }

    fn stop_specific_outputs(&mut self) {
        let mut outputs: Vec<String> = self
            .processes
            .keys()
            .chain(self.sockets.keys())
            .filter(|out| out.as_str() != "*")
            .cloned()
            .collect();
        outputs.sort();
        outputs.dedup();
        for out in outputs {
            self.stop_output(&out);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_wallpaper(
        &mut self,
        output: &str,
        video_path: &str,
        scaling: &str,
        mute: bool,
        volume: u8,
        hwdec: &str,
        auto_pause: bool,
        gpu_pref: &str,
        force_spawn: bool,
        retire_overrides: bool,
    ) -> Result<u32, std::io::Error> {
        let is_image = std::path::Path::new(video_path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| crate::scanner::IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false);
        let effective_path = if is_image {
            crate::scanner::thumbs::optimize_wallpaper_image(std::path::Path::new(video_path), 7680, 4320)
        } else {
            std::path::PathBuf::from(video_path)
        };
        let effective_path_str = effective_path.to_string_lossy().to_string();

        self.ensure_sockets_discovered();
        let active_sock = self
            .sockets
            .get(output)
            .cloned()
            .unwrap_or_else(|| socket_path_for_output(output));

        // Fast path: reuse the live mpv instance via IPC. For a wildcard apply
        // this preserves the default surface underneath every per-output override.
        let mut can_reuse = !force_spawn;
        if can_reuse {
            if let Some(child) = self.processes.get_mut(output) {
                can_reuse = matches!(child.try_wait(), Ok(None)) && ipc_is_alive(&active_sock);
            } else {
                can_reuse = ipc_is_alive(&active_sock);
            }
        }

        if can_reuse {
            let _ = ipc_set_property(&active_sock, "mute", mute);
            let _ = ipc_set_property(&active_sock, "volume", volume.min(100));
            let _ = ipc_set_property(&active_sock, "pause", false);
            match scaling {
                "fill" => {
                    let _ = ipc_set_property(&active_sock, "panscan", 1.0f32);
                    let _ = ipc_set_property(&active_sock, "keepaspect", true);
                }
                "stretch" => {
                    let _ = ipc_set_property(&active_sock, "panscan", 0.0f32);
                    let _ = ipc_set_property(&active_sock, "keepaspect", false);
                }
                _ => {
                    let _ = ipc_set_property(&active_sock, "panscan", 0.0f32);
                    let _ = ipc_set_property(&active_sock, "keepaspect", true);
                }
            }

            if ipc_loadfile(&active_sock, &effective_path_str).is_ok() {
                self.is_paused = false;
                self.output_is_image.insert(output.to_string(), is_image);
                self.current_paths
                    .insert(output.to_string(), video_path.to_string());
                self.sockets.insert(output.to_string(), active_sock.clone());

                // The new default is visible. Only now retire overrides that
                // would otherwise hide it. Hotplug restarts preserve overrides.
                if retire_overrides {
                    self.stop_specific_outputs();
                }

                if let Some(child) = self.processes.get(output) {
                    return Ok(child.id());
                }
                if let Some(pid) = ipc_get_pid(&active_sock) {
                    return Ok(pid);
                }
                return Ok(0);
            }
        }

        // Transactional spawn: the old surface is deliberately left untouched
        // until the new process has an IPC socket and has presented its first frame.
        let final_sock = socket_path_for_output(output);
        let staging_sock = final_sock.with_extension("next.sock");
        let _ = std::fs::remove_file(&staging_sock);

        let mut opts = Self::build_mpv_options_with_gpu(scaling, mute, volume, hwdec, is_image, gpu_pref);
        opts.push_str(&format!(" --input-ipc-server={}", staging_sock.display()));

        let mpv_bin = resolve_mpvpaper_binary();
        let use_systemd_run =
            can_use_systemd_run() && !crate::engine::gpu::systemd_run_gpu_args(gpu_pref).is_empty();

        let build_command = |socket_path: &std::path::Path| {
            let mut cmd = if use_systemd_run {
                let mut c = Command::new("systemd-run");
                c.arg("--user").arg("--scope").arg("--quiet");
                for arg in crate::engine::gpu::systemd_run_gpu_args(gpu_pref) {
                    c.arg(arg);
                }
                c.arg(&mpv_bin);
                c
            } else {
                Command::new(&mpv_bin)
            };
            crate::engine::gpu::apply_gpu_env(&mut cmd, gpu_pref);
            cmd.arg("-l").arg("background");
            if auto_pause {
                cmd.arg("-p");
            }
            let mut opts = opts.clone();
            opts = opts.replace(
                &format!("--input-ipc-server={}", staging_sock.display()),
                &format!("--input-ipc-server={}", socket_path.display()),
            );
            cmd.arg("-o")
                .arg(&opts)
                .arg(output)
                .arg(&effective_path_str)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());

            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                unsafe {
                    cmd.pre_exec(|| {
                        libc::setsid();
                        libc::signal(libc::SIGHUP, libc::SIG_IGN);
                        Ok(())
                    });
                }
            }
            cmd
        };

        let mut child = match build_command(&staging_sock).spawn() {
            Ok(c) => c,
            Err(_e) if use_systemd_run => {
                let mut fallback = build_command(&staging_sock);
                // systemd-run is unavailable: remove the wrapper by rebuilding
                // the command directly below when possible.
                let _ = &mut fallback;
                let mut direct = Command::new(&mpv_bin);
                crate::engine::gpu::apply_gpu_env(&mut direct, gpu_pref);
                direct.arg("-l").arg("background");
                if auto_pause {
                    direct.arg("-p");
                }
                direct
                    .arg("-o")
                    .arg(&opts)
                    .arg(output)
                    .arg(&effective_path_str)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                #[cfg(unix)]
                {
                    use std::os::unix::process::CommandExt;
                    unsafe {
                        direct.pre_exec(|| {
                            libc::setsid();
                            libc::signal(libc::SIGHUP, libc::SIG_IGN);
                            Ok(())
                        });
                    }
                }
                match direct.spawn() {
                    Ok(c) => c,
                    Err(fe) => {
                        let _ = std::fs::remove_file(&staging_sock);
                        let (distro, cmd_str) = detect_distro_command();
                        let err_msg = if fe.kind() == std::io::ErrorKind::NotFound {
                            format!("No se encontró 'mpvpaper'. En {}, instala las dependencias con: {}", distro, cmd_str)
                        } else {
                            format!("Error al iniciar mpvpaper: {} (output: {}, video: {})", fe, output, video_path)
                        };
                        eprintln!("[Aura Engine] {}", err_msg);
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, err_msg));
                    }
                }
            }
            Err(fe) => {
                let _ = std::fs::remove_file(&staging_sock);
                let (distro, cmd_str) = detect_distro_command();
                let err_msg = if fe.kind() == std::io::ErrorKind::NotFound {
                    format!("No se encontró 'mpvpaper'. En {}, instala las dependencias con: {}", distro, cmd_str)
                } else {
                    format!("Error al iniciar mpvpaper: {} (output: {}, video: {})", fe, output, video_path)
                };
                eprintln!("[Aura Engine] {}", err_msg);
                return Err(std::io::Error::new(std::io::ErrorKind::Other, err_msg));
            }
        };

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut exited_status = None;
        let mut ready = false;
        while Instant::now() < deadline {
            if ipc_is_alive(&staging_sock) {
                ready = true;
                break;
            }
            if let Ok(Some(status)) = child.try_wait() {
                exited_status = Some(status);
                if !use_systemd_run {
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        if !ready {
            let (distro, cmd_str) = detect_distro_command();
            let err_msg = match exited_status {
                Some(status) if status.code() == Some(127) => format!(
                    "mpvpaper falló al cargar librerías multimedia (código 127). En {}, ejecuta: {}",
                    distro, cmd_str
                ),
                Some(status) => format!(
                    "mpvpaper terminó antes de iniciar el IPC (código {:?}). Verifica el formato y dependencias multimedia.",
                    status.code()
                ),
                None => "mpvpaper no inició su socket IPC a tiempo. Verifica el formato y las dependencias multimedia.".to_string(),
            };
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&staging_sock);
            eprintln!("[Aura Engine] {}", err_msg);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, err_msg));
        }

        // Loading a large image/video can take a little longer while the
        // previous surface is still decoding. Give the staged surface three
        // seconds to present a frame before rolling back.
        if !ipc_wait_first_frame(
            &staging_sock,
            Some(Path::new(&effective_path_str)),
            Duration::from_secs(3),
        ) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&staging_sock);
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "mpvpaper no presentó el primer frame a tiempo; se mantiene el fondo anterior",
            ));
        }

        let pid = match ipc_get_pid(&staging_sock)
            .filter(|pid| is_mpvpaper_pid(*pid))
            .or_else(|| {
                let child_pid = child.id();
                is_mpvpaper_pid(child_pid).then_some(child_pid)
            }) {
            Some(pid) => pid,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&staging_sock);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No se pudo obtener el PID real de mpvpaper",
                ));
            }
        };

        // New surface is ready and playing. Retire only the old surface for
        // this target, then make its socket the canonical one.
        let had_old = self.processes.contains_key(output) || self.sockets.contains_key(output);
        if had_old {
            self.stop_output(output);
        }
        let _ = std::fs::remove_file(&final_sock);
        let active_sock = match std::fs::rename(&staging_sock, &final_sock) {
            Ok(()) => final_sock,
            Err(e) => {
                eprintln!(
                    "[Aura Engine] No se pudo renombrar el socket de transición: {}. Se usará la ruta temporal.",
                    e
                );
                staging_sock
            }
        };

        self.processes.insert(output.to_string(), child);
        self.sockets.insert(output.to_string(), active_sock);
        self.output_is_image.insert(output.to_string(), is_image);
        self.current_paths
            .insert(output.to_string(), video_path.to_string());
        self.is_paused = false;

        if retire_overrides {
            self.stop_specific_outputs();
        }

        Ok(pid)
    }

    // GNOME has a single desktop background, so every output maps to "*".
    fn set_wallpaper_gnome(&mut self, video_path: &str, scaling: &str, mute: bool, volume: u8) -> Result<u32, std::io::Error> {
        let path = std::path::Path::new(video_path);
        let is_image = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| crate::scanner::IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false);
        let result = if is_image {
            crate::engine::gnome::set_image(path, scaling)
        } else {
            crate::engine::gnome::set_video(path, scaling, mute, volume)
        };
        result.map_err(|e| {
            eprintln!("[Aura Engine] {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, e)
        })?;
        self.output_is_image.clear();
        self.output_is_image.insert("*".to_string(), is_image);
        self.is_paused = false;
        Ok(0)
    }

    pub fn set_volume(&mut self, output: Option<&str>, volume: u8) -> bool {
        if crate::engine::gnome::is_active() {
            return crate::engine::gnome::set_volume(volume);
        }
        self.ensure_sockets_discovered();
        let mut success = false;
        let v = volume.min(100);
        if let Some(out) = output {
            if let Some(sock) = self.sockets.get(out) {
                if ipc_set_property(sock, "volume", v).is_ok() {
                    success = true;
                }
            }
        } else {
            for sock in self.sockets.values() {
                if ipc_set_property(sock, "volume", v).is_ok() {
                    success = true;
                }
            }
        }
        success
    }

    pub fn set_mute(&mut self, output: Option<&str>, mute: bool) -> bool {
        if crate::engine::gnome::is_active() {
            return crate::engine::gnome::set_mute(mute);
        }
        self.ensure_sockets_discovered();
        let mut success = false;
        if let Some(out) = output {
            if let Some(sock) = self.sockets.get(out) {
                if ipc_set_property(sock, "mute", mute).is_ok() {
                    success = true;
                }
            }
        } else {
            for sock in self.sockets.values() {
                if ipc_set_property(sock, "mute", mute).is_ok() {
                    success = true;
                }
            }
        }
        success
    }

    pub fn set_scaling(&mut self, output: &str, scaling: &str) -> bool {
        if crate::engine::gnome::is_active() {
            return crate::engine::gnome::set_scaling(scaling);
        }
        self.ensure_sockets_discovered();
        if let Some(sock) = self.sockets.get(output) {
            match scaling {
                "fill" => {
                    let _ = ipc_set_property(sock, "panscan", 1.0f32);
                    let _ = ipc_set_property(sock, "keepaspect", true);
                }
                "stretch" => {
                    let _ = ipc_set_property(sock, "panscan", 0.0f32);
                    let _ = ipc_set_property(sock, "keepaspect", false);
                }
                _ => {
                    let _ = ipc_set_property(sock, "panscan", 0.0f32);
                    let _ = ipc_set_property(sock, "keepaspect", true);
                }
            }
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        !self.processes.is_empty()
    }

    #[allow(dead_code)]
    pub fn is_any_paused(&mut self) -> bool {
        self.ensure_sockets_discovered();
        for sock in self.sockets.values() {
            if ipc_get_pause(sock) == Some(true) {
                return true;
            }
        }
        self.own_pids().iter().any(|&pid| is_pid_stopped(pid))
    }

    /// Reads the effective pause state for an output. A per-output player takes
    /// precedence over the shared wildcard player.
    pub fn output_pause_state(&mut self, output: &str) -> Option<bool> {
        if crate::engine::gnome::is_active() {
            return Some(self.is_paused);
        }
        self.ensure_sockets_discovered();
        let sock = self.sockets.get(output).or_else(|| self.sockets.get("*"))?;
        ipc_get_pause(sock)
    }

    pub fn pause_states_for_outputs(&mut self, outputs: &[String]) -> HashMap<String, Option<bool>> {
        self.ensure_sockets_discovered();
        let mut queried = HashMap::<PathBuf, Option<bool>>::new();
        let mut states = HashMap::new();
        for output in outputs {
            let state = if crate::engine::gnome::is_active() {
                Some(self.is_paused)
            } else if let Some(sock) = self.sockets.get(output).or_else(|| self.sockets.get("*")) {
                *queried.entry(sock.clone()).or_insert_with(|| ipc_get_pause(sock))
            } else {
                None
            };
            states.insert(output.clone(), state);
        }
        let wildcard = self.sockets.get("*").and_then(|sock| {
            *queried.entry(sock.clone()).or_insert_with(|| ipc_get_pause(sock))
        });
        states.insert("*".into(), wildcard);
        states
    }

    /// Sets the pause state for one output without cycling unrelated players.
    /// If the output has no dedicated player, its wildcard player is used.
    pub fn set_output_paused(&mut self, output: &str, paused: bool) -> bool {
        if crate::engine::gnome::is_active() {
            if paused {
                crate::engine::gnome::pause();
            } else {
                crate::engine::gnome::play();
            }
            self.is_paused = paused;
            return true;
        }
        self.ensure_sockets_discovered();
        let Some(sock) = self.sockets.get(output).or_else(|| self.sockets.get("*")).cloned() else {
            return false;
        };
        let pid = ipc_get_pid(&sock).filter(|pid| is_mpvpaper_pid(*pid));
        if !paused {
            if let Some(pid) = pid.filter(|pid| is_pid_stopped(*pid)) {
                unsafe { libc::kill(pid as i32, libc::SIGCONT); }
            }
        }
        let success = if ipc_set_property(&sock, "pause", paused).is_ok() {
            true
        } else if paused {
            pid.is_some_and(|pid| unsafe { libc::kill(pid as i32, libc::SIGSTOP) == 0 })
        } else {
            false
        };
        if output == "*" {
            self.is_paused = paused;
        }
        success
    }

    /// Sets every currently known player to the same state.
    pub fn set_all_paused(&mut self, paused: bool) -> bool {
        if paused {
            self.pause_all();
        } else {
            self.resume_all();
        }
        self.is_paused = paused;
        true
    }

    pub fn toggle_pause(&mut self) -> bool {
        if crate::engine::gnome::is_active() {
            if self.is_paused {
                self.resume_all();
            } else {
                self.pause_all();
            }
            return self.is_paused;
        }

        // Ensure sockets has active sockets if started by an external or previous instance
        self.ensure_sockets_discovered();

        let states: Vec<bool> = self.sockets.values().filter_map(|sock| ipc_get_pause(sock)).collect();
        if !states.is_empty() {
            // Normalize mixed players in one direction instead of inverting
            // each process independently (which can preserve a mixed state).
            let now_paused = !states.iter().any(|paused| *paused);
            if now_paused {
                self.pause_all();
            } else {
                self.resume_all();
            }
            return self.is_paused;
        }

        // Fallback: check only Aura-tracked PIDs and toggle via POSIX signals
        let pids = self.own_pids();
        if pids.is_empty() {
            return self.is_paused;
        }
        let any_stopped = pids.iter().any(|&pid| is_pid_stopped(pid));

        if any_stopped {
            self.resume_all();
            false
        } else {
            self.pause_all();
            true
        }
    }

    pub fn pause_all(&mut self) {
        if crate::engine::gnome::is_active() {
            crate::engine::gnome::pause();
            self.is_paused = true;
            return;
        }
        self.ensure_sockets_discovered();
        // 1. Send pause via IPC
        let mut ipc_controlled = std::collections::HashSet::new();
        for sock in self.sockets.values() {
            if ipc_set_property(sock, "pause", true).is_ok() {
                if let Some(pid) = ipc_get_pid(sock).filter(|pid| is_mpvpaper_pid(*pid)) {
                    ipc_controlled.insert(pid);
                }
            }
        }
        for pid in self.own_pids() {
            if !ipc_controlled.contains(&pid) {
                unsafe { libc::kill(pid as i32, libc::SIGSTOP); }
            }
        }
        self.is_paused = true;
    }

    pub fn resume_all(&mut self) {
        if crate::engine::gnome::is_active() {
            crate::engine::gnome::play();
            self.is_paused = false;
            return;
        }
        self.ensure_sockets_discovered();
        // Send unpause through IPC. Keeping the process responsive lets the
        // UI query each output's actual state while paused.
        for pid in self.own_pids() {
            if is_pid_stopped(pid) {
                unsafe { libc::kill(pid as i32, libc::SIGCONT); }
            }
        }
        for sock in self.sockets.values() {
            let _ = ipc_set_property(sock, "pause", false);
        }
        self.is_paused = false;
    }

    pub fn stop_output(&mut self, output: &str) {
        if crate::engine::gnome::is_active() {
            crate::engine::gnome::stop();
            self.sockets.clear();
            self.output_is_image.clear();
            self.current_paths.clear();
            return;
        }
        let sock_path = self
            .sockets
            .get(output)
            .cloned()
            .unwrap_or_else(|| socket_path_for_output(output));
        let status = ipc_status(&sock_path);
        if matches!(status, IpcStatus::Alive | IpcStatus::Unknown) {
            let _ = send_ipc_command(&sock_path, r#"{"command": ["quit"]}"#);
        }

        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline && !matches!(ipc_status(&sock_path), IpcStatus::Dead) {
            std::thread::sleep(Duration::from_millis(25));
        }

        if !matches!(ipc_status(&sock_path), IpcStatus::Dead) {
            if let Some(pid) = pid_for_socket(&sock_path).filter(|pid| is_mpvpaper_pid(*pid)) {
                unsafe {
                    libc::kill(pid as i32, libc::SIGCONT);
                    libc::kill(pid as i32, libc::SIGTERM);
                }
                let deadline = Instant::now() + Duration::from_secs(1);
                while Instant::now() < deadline
                    && !matches!(ipc_status(&sock_path), IpcStatus::Dead)
                {
                    std::thread::sleep(Duration::from_millis(25));
                }
                if !matches!(ipc_status(&sock_path), IpcStatus::Dead) {
                    unsafe {
                        libc::kill(pid as i32, libc::SIGKILL);
                    }
                }
            }
        }

        if let Some(mut child) = self.processes.remove(output) {
            if let Ok(None) = child.try_wait() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }

        let socket_has_process = pid_for_socket(&sock_path).is_some();
        if matches!(ipc_status(&sock_path), IpcStatus::Dead) || !socket_has_process {
            let _ = std::fs::remove_file(&sock_path);
        }
        self.sockets.remove(output);
        self.output_is_image.remove(output);
        self.current_paths.remove(output);
    }

    pub fn stop_all(&mut self) {
        if crate::engine::gnome::is_active() {
            crate::engine::gnome::stop();
            self.processes.clear();
            self.sockets.clear();
            self.output_is_image.clear();
            self.current_paths.clear();
            self.is_paused = false;
            return;
        }

        self.ensure_sockets_discovered();
        let mut outputs: Vec<String> = self
            .sockets
            .keys()
            .chain(self.processes.keys())
            .chain(self.current_paths.keys())
            .cloned()
            .collect();
        outputs.sort();
        outputs.dedup();
        for output in outputs {
            self.stop_output(&output);
        }
        self.output_is_image.clear();
        self.current_paths.clear();
        self.is_paused = false;
    }

    #[allow(dead_code)]
    pub fn build_mpv_options(scaling: &str, mute: bool, volume: u8, hwdec: &str, is_image: bool) -> String {
        Self::build_mpv_options_with_gpu(scaling, mute, volume, hwdec, is_image, "auto")
    }

    pub fn build_mpv_options_with_gpu(scaling: &str, mute: bool, volume: u8, hwdec: &str, _is_image: bool, gpu_pref: &str) -> String {
        let shader_cache = shader_cache_dir();
        let mut opts = format!(
            "loop-file=inf --image-display-duration=inf --hwdec={} --no-config --demuxer-max-bytes=16M --demuxer-readahead-secs=2 --vd-lavc-threads=2 --vd-lavc-dr=yes --gpu-shader-cache-dir={} --sub-auto=no --audio-display=no --no-osc --no-osd-bar --osd-level=0 --hr-seek=no --background-color=#000000",
            hwdec,
            shader_cache.display()
        );

        if (hwdec == "vaapi" || hwdec == "auto-safe") && gpu_pref != "auto" {
            let gpus = crate::engine::gpu::detect_available_gpus();
            if let Some(dev) = crate::engine::gpu::resolve_vaapi_device(gpu_pref, &gpus) {
                opts.push_str(&format!(" --vaapi-device={}", dev));
            }
        }

        opts.push_str(if mute { " --mute=yes" } else { " --mute=no" });
        opts.push_str(&format!(" --volume={}", volume.min(100)));

        match scaling {
            "fill" => opts.push_str(" --panscan=1.0"),
            "stretch" => opts.push_str(" --no-keepaspect"),
            _ => {} // fit is default
        }
        opts
    }

    pub fn autostart_dir() -> PathBuf {
        crate::config::config_home().join("autostart")
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
            (
                "flatpak run io.github.antwny.aura --hidden".to_string(),
                "flatpak".to_string(),
            )
        } else {
            let home = std::env::var("HOME").unwrap_or_default();
            let user_bin = PathBuf::from(&home).join(".local/bin/aura");
            let path = if user_bin.exists() {
                Some(user_bin)
            } else {
                std::env::current_exe().ok()
            };
            match path {
                Some(path) => (
                    format!("{} --hidden", desktop_exec_quote(&path.to_string_lossy())),
                    path.to_string_lossy().to_string(),
                ),
                None => ("aura --hidden".to_string(), "aura".to_string()),
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

pub fn desktop_exec_quote(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$");
    format!("\"{}\"", escaped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_mpv_options() {
        let opts_fit_mute = WallpaperEngine::build_mpv_options("fit", true, 100, "auto-safe", false);
        assert!(opts_fit_mute.contains("--hwdec=auto-safe"));
        assert!(opts_fit_mute.contains("--mute=yes"));
        assert!(opts_fit_mute.contains("--volume=100"));
        assert!(!opts_fit_mute.contains("--no-audio"));
        assert!(opts_fit_mute.contains("--demuxer-max-bytes=16M"));
        assert!(opts_fit_mute.contains("--demuxer-readahead-secs=2"));
        assert!(opts_fit_mute.contains("--vd-lavc-threads=2"));
        assert!(opts_fit_mute.contains("--vd-lavc-dr=yes"));
        assert!(opts_fit_mute.contains("--gpu-shader-cache-dir="));
        assert!(opts_fit_mute.contains("--sub-auto=no"));
        assert!(opts_fit_mute.contains("--no-osc"));
        assert!(opts_fit_mute.contains("--no-osd-bar"));
        assert!(opts_fit_mute.contains("--osd-level=0"));
        assert!(opts_fit_mute.contains("--hr-seek=no"));
        assert!(opts_fit_mute.contains("--background-color=#000000"));
        assert!(!opts_fit_mute.contains("--panscan"));

        let opts_fill_sound = WallpaperEngine::build_mpv_options("fill", false, 80, "vaapi", false);
        assert!(opts_fill_sound.contains("--hwdec=vaapi"));
        assert!(opts_fill_sound.contains("--mute=no"));
        assert!(!opts_fill_sound.contains("--no-audio"));
        assert!(opts_fill_sound.contains("--volume=80"));
        assert!(opts_fill_sound.contains("--panscan=1.0"));

        let opts_stretch = WallpaperEngine::build_mpv_options("stretch", false, 100, "nvdec", false);
        assert!(opts_stretch.contains("--no-keepaspect"));

        let opts_image = WallpaperEngine::build_mpv_options("fill", true, 100, "auto-safe", true);
        assert!(opts_image.contains("image-display-duration=inf"));
        assert!(opts_image.contains("--hwdec=auto-safe"));
        assert!(opts_image.contains("--mute=yes"));
        assert!(opts_image.contains("--demuxer-max-bytes=16M"));
        assert!(opts_image.contains("--vd-lavc-threads=2"));
        assert!(opts_image.contains("--vd-lavc-dr=yes"));
        assert!(opts_image.contains("--panscan=1.0"));
    }

    #[test]
    fn test_build_mpv_options_with_gpu() {
        let opts_auto = WallpaperEngine::build_mpv_options_with_gpu("fit", true, 100, "vaapi", false, "auto");
        assert!(opts_auto.contains("--hwdec=vaapi"));

        let opts_discrete = WallpaperEngine::build_mpv_options_with_gpu("fit", true, 100, "vaapi", false, "discrete");
        assert!(opts_discrete.contains("--hwdec=vaapi"));

        let opts_custom = WallpaperEngine::build_mpv_options_with_gpu("fit", true, 100, "vaapi", false, "/dev/dri/renderD129");
        assert!(opts_custom.contains("--vaapi-device=/dev/dri/renderD129"));
    }

    #[test]
    fn test_shell_escape_safety() {
        assert_eq!(shell_escape("normal_string"), "'normal_string'");
        assert_eq!(shell_escape("with space"), "'with space'");
        assert_eq!(shell_escape("with$dollar"), "'with$dollar'");
        assert_eq!(shell_escape("with`backtick`"), "'with`backtick`'");
        assert_eq!(shell_escape("with'quote"), "'with'\\''quote'");
    }

    #[test]
    fn test_desktop_exec_quote() {
        assert_eq!(desktop_exec_quote("/home/user/Aura"), "\"/home/user/Aura\"");
        assert_eq!(
            desktop_exec_quote("/home/user/My Aura/aura"),
            "\"/home/user/My Aura/aura\""
        );
        assert_eq!(
            desktop_exec_quote("/tmp/a\"b$c`d\\e"),
            "\"/tmp/a\\\"b\\$c\\`d\\\\e\""
        );
    }

    #[test]
    fn test_socket_paths() {
        let sock_default = socket_path_for_output("*");
        assert!(sock_default.to_string_lossy().ends_with("mpv_all.sock"));
        assert_eq!(output_from_socket_stem("mpv_all"), "*");
        assert_eq!(output_from_socket_stem("all"), "*");

        let sock_edp = socket_path_for_output("eDP-1");
        assert!(sock_edp.to_string_lossy().ends_with("mpv_eDP-1.sock"));
        assert_eq!(output_from_socket_stem("mpv_eDP-1"), "eDP-1");

        let sock_special = socket_path_for_output("HDMI-A-1/test");
        assert!(sock_special.to_string_lossy().ends_with("mpv_HDMI-A-1test.sock"));
        assert_eq!(output_from_socket_stem("mpv_HDMI-A-1test"), "HDMI-A-1test");
    }

    #[test]
    fn test_engine_init() {
        let mut engine = WallpaperEngine::new();
        assert!(!engine.is_paused);
        assert!(!engine.is_running());
        assert!(!engine.is_output_running("eDP-1"));
    }

    #[test]
    fn test_is_pid_stopped() {
        let my_pid = std::process::id();
        assert!(!is_pid_stopped(my_pid));
        assert!(!is_pid_stopped(9999999));
    }

    #[test]
    fn test_detect_distro_command() {
        let (distro, cmd) = detect_distro_command();
        assert!(!distro.is_empty());
        assert!(!cmd.is_empty());
    }

    #[test]
    fn test_detect_os_pretty_name() {
        let os = detect_os_pretty_name();
        assert!(!os.is_empty());
    }

    #[test]
    fn test_can_use_systemd_run() {
        // Must return bool without panicking
        let _ = can_use_systemd_run();
    }

    #[test]
    fn test_ipc_get_pid_non_existent() {
        let dummy = PathBuf::from("/tmp/non_existent_sock_123456.sock");
        assert_eq!(ipc_get_pid(&dummy), None);
    }

    #[test]
    fn test_ipc_status_missing_socket_is_dead() {
        let dummy = PathBuf::from("/tmp/non_existent_sock_123456.sock");
        assert_eq!(ipc_status(&dummy), IpcStatus::Dead);
        assert!(!ipc_is_alive(&dummy));
    }

    #[test]
    fn test_previous_wallpapers_never_restores_over_wildcard() {
        let mut engine = WallpaperEngine::new();
        engine
            .current_paths
            .insert("*".to_string(), "/tmp/all.mp4".to_string());
        engine
            .current_paths
            .insert("eDP-1".to_string(), "/tmp/old.mp4".to_string());

        let previous = engine.previous_wallpapers("eDP-1");
        assert!(!previous.contains(&("*".to_string(), "/tmp/all.mp4".to_string())));
        assert!(previous.contains(&("eDP-1".to_string(), "/tmp/old.mp4".to_string())));

        let mut all = engine.previous_wallpapers("*");
        all.sort();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_own_pids_ignores_foreign_processes() {
        let engine = WallpaperEngine::new();
        assert!(engine.own_pids().is_empty());
        assert!(!is_mpvpaper_pid(std::process::id()));
    }

    #[test]
    fn test_reap_and_processes_keys() {
        let mut engine = WallpaperEngine::new();
        assert!(engine.processes_keys().is_empty());
        let dead = engine.reap_dead_processes();
        assert!(dead.is_empty());
    }

    #[test]
    fn test_output_is_image_tracking() {
        let mut engine = WallpaperEngine::new();
        assert!(engine.output_is_image.is_empty());

        engine.output_is_image.insert("eDP-1".to_string(), true);
        engine.output_is_image.insert("HDMI-A-1".to_string(), false);
        assert_eq!(engine.output_is_image.get("eDP-1"), Some(&true));
        assert_eq!(engine.output_is_image.get("HDMI-A-1"), Some(&false));

        engine.stop_output("eDP-1");
        assert_eq!(engine.output_is_image.get("eDP-1"), None);
        assert_eq!(engine.output_is_image.get("HDMI-A-1"), Some(&false));

        engine.stop_all();
        assert!(engine.output_is_image.is_empty());
    }
}

