use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
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

pub fn socket_dir() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_RUNTIME_DIR") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("aura");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".cache/aura/ipc")
}

pub fn socket_path_for_output(output: &str) -> PathBuf {
    let clean: String = output.chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
    let name = if clean.is_empty() { "all".to_string() } else { clean };
    let dir = socket_dir();
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("mpv_{}.sock", name))
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

pub fn ipc_is_alive(sock_path: &Path) -> bool {
    if !sock_path.exists() {
        return false;
    }
    match send_ipc_command(sock_path, r#"{"command": ["get_property", "pause"]}"#) {
        Ok(resp) => resp.contains("error") && resp.contains("success"),
        Err(_) => {
            let _ = std::fs::remove_file(sock_path);
            false
        }
    }
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

pub fn ipc_cycle_pause(sock_path: &Path) -> Result<bool, String> {
    let cycle_cmd = r#"{"command": ["cycle", "pause"]}"#;
    let _ = send_ipc_command(sock_path, cycle_cmd)?;

    let get_cmd = r#"{"command": ["get_property", "pause"]}"#;
    let resp = send_ipc_command(sock_path, get_cmd)?;
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&resp) {
        if let Some(b) = val.get("data").and_then(|d| d.as_bool()) {
            return Ok(b);
        }
    }
    Ok(false)
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
    pub is_paused: bool,
}

impl WallpaperEngine {
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
            sockets: HashMap::new(),
            is_paused: false,
        }
    }

    pub fn processes_keys(&self) -> Vec<String> {
        self.processes.keys().cloned().collect()
    }

    pub fn reap_dead_processes(&mut self) -> Vec<String> {
        let mut dead = Vec::new();
        self.processes.retain(|output, child| {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    dead.push(output.clone());
                    false
                }
                Ok(None) => true,
                Err(_) => {
                    dead.push(output.clone());
                    false
                }
            }
        });
        for out in &dead {
            if let Some(sock) = self.sockets.remove(out) {
                let _ = std::fs::remove_file(sock);
            }
        }
        dead
    }

    pub fn ensure_sockets_discovered(&mut self) {
        if self.sockets.is_empty() {
            let sock_dir = socket_dir();
            if let Ok(entries) = std::fs::read_dir(&sock_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let p = entry.path();
                    if p.extension().and_then(|e| e.to_str()) == Some("sock") {
                        if ipc_is_alive(&p) {
                            let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
                            let out_name = name.strip_prefix("mpv_").unwrap_or(name);
                            self.sockets.insert(out_name.to_string(), p);
                        }
                    }
                }
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
    ) -> Result<u32, std::io::Error> {
        // If applying to all monitors ("*"), stop any specific outputs first
        if output == "*" {
            self.stop_all();
        } else {
            // Stop wildcard if running, and stop this specific output's previous process
            self.stop_output("*");
        }

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

        let sock_path = socket_path_for_output(output);

        // Fast, seamless transition via IPC if mpvpaper is already running on this output!
        let mut can_reuse = false;
        if let Some(child) = self.processes.get_mut(output) {
            if let Ok(None) = child.try_wait() {
                if ipc_is_alive(&sock_path) {
                    can_reuse = true;
                }
            }
        } else if ipc_is_alive(&sock_path) {
            can_reuse = true;
        }

        if can_reuse {
            let _ = ipc_set_property(&sock_path, "mute", mute);
            let _ = ipc_set_property(&sock_path, "volume", volume.min(100));
            match scaling {
                "fill" => {
                    let _ = ipc_set_property(&sock_path, "panscan", 1.0f32);
                    let _ = ipc_set_property(&sock_path, "keepaspect", true);
                }
                "stretch" => {
                    let _ = ipc_set_property(&sock_path, "panscan", 0.0f32);
                    let _ = ipc_set_property(&sock_path, "keepaspect", false);
                }
                _ => {
                    let _ = ipc_set_property(&sock_path, "panscan", 0.0f32);
                    let _ = ipc_set_property(&sock_path, "keepaspect", true);
                }
            }

            if ipc_loadfile(&sock_path, &effective_path_str).is_ok() {
                self.is_paused = false;
                self.sockets.insert(output.to_string(), sock_path.clone());
                if let Some(child) = self.processes.get(output) {
                    return Ok(child.id());
                } else if let Some(pid) = ipc_get_pid(&sock_path) {
                    return Ok(pid);
                } else {
                    return Ok(0);
                }
            }
        }

        // If not reusable, clean up previous process and socket
        self.stop_output(output);
        let _ = std::fs::remove_file(&sock_path);

        let mut opts = Self::build_mpv_options(scaling, mute, volume, hwdec, is_image);
        opts.push_str(&format!(" --input-ipc-server={}", sock_path.display()));

        let mpv_bin = resolve_mpvpaper_binary();
        let use_systemd_run = can_use_systemd_run();

        let mut cmd = if use_systemd_run {
            let mut c = Command::new("systemd-run");
            c.arg("--user")
                .arg("--scope")
                .arg("--quiet")
                .arg(&mpv_bin);
            c
        } else {
            Command::new(&mpv_bin)
        };

        // Run mpvpaper on `background` layer so COSMIC desktop icons (`cosmic-files-applet`)
        // and right-click desktop interactions remain fully accessible on the `bottom` layer.
        cmd.arg("-l").arg("background");

        if auto_pause {
            cmd.arg("-p");
        }

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

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                if use_systemd_run {
                    let mut fallback = Command::new(&mpv_bin);
                    fallback.arg("-l").arg("background");
                    if auto_pause {
                        fallback.arg("-p");
                    }
                    fallback.arg("-o")
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
                            fallback.pre_exec(|| {
                                libc::setsid();
                                libc::signal(libc::SIGHUP, libc::SIG_IGN);
                                Ok(())
                            });
                        }
                    }
                    match fallback.spawn() {
                        Ok(c) => c,
                        Err(fe) => {
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
                } else {
                    let (distro, cmd_str) = detect_distro_command();
                    let err_msg = if e.kind() == std::io::ErrorKind::NotFound {
                        format!("No se encontró 'mpvpaper'. En {}, instala las dependencias con: {}", distro, cmd_str)
                    } else {
                        format!("Error al iniciar mpvpaper: {} (output: {}, video: {})", e, output, video_path)
                    };
                    eprintln!("[Aura Engine] {}", err_msg);
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, err_msg));
                }
            }
        };

        // Give dynamic linker 25ms to verify process didn't terminate immediately (e.g. exit 127: missing libmpv)
        std::thread::sleep(std::time::Duration::from_millis(25));
        if let Ok(Some(status)) = child.try_wait() {
            let (distro, cmd_str) = detect_distro_command();
            let err_msg = if status.code() == Some(127) {
                format!(
                    "mpvpaper falló al cargar librerías multimedia (código 127). En {}, ejecuta: {}",
                    distro, cmd_str
                )
            } else {
                format!(
                    "mpvpaper terminó inmediatamente con código {:?}. Verifica el formato y dependencias multimedia.",
                    status.code()
                )
            };
            eprintln!("[Aura Engine] {}", err_msg);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, err_msg));
        }

        let pid = child.id();
        self.processes.insert(output.to_string(), child);
        self.sockets.insert(output.to_string(), sock_path);
        self.is_paused = false;
        Ok(pid)
    }

    pub fn set_volume(&mut self, output: Option<&str>, volume: u8) -> bool {
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
    pub fn is_any_paused(&self) -> bool {
        for sock in self.sockets.values() {
            if let Some(p) = ipc_get_pause(sock) {
                return p;
            }
        }
        let pids = find_mpvpaper_pids();
        pids.iter().any(|&pid| is_pid_stopped(pid))
    }

    pub fn toggle_pause(&mut self) -> bool {
        // Ensure sockets has active sockets if started by an external or previous instance
        self.ensure_sockets_discovered();

        let mut any_toggled = false;
        let mut now_paused = false;

        for sock in self.sockets.values() {
            if let Ok(paused) = ipc_cycle_pause(sock) {
                any_toggled = true;
                now_paused = paused;
            }
        }

        if any_toggled {
            self.is_paused = now_paused;
            return now_paused;
        }

        // Fallback: check /proc for stopped state and toggle via POSIX signals
        let pids = find_mpvpaper_pids();
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
        // 1. Send pause via IPC
        for sock in self.sockets.values() {
            let _ = ipc_set_property(sock, "pause", true);
        }
        // 2. Also send SIGSTOP to freeze CPU/GPU via OS scheduler
        let pids = find_mpvpaper_pids();
        for pid in pids {
            unsafe {
                libc::kill(pid as i32, libc::SIGSTOP);
            }
        }
        self.is_paused = true;
    }

    pub fn resume_all(&mut self) {
        // 1. Send SIGCONT to unfreeze OS execution
        let pids = find_mpvpaper_pids();
        for pid in pids {
            unsafe {
                libc::kill(pid as i32, libc::SIGCONT);
            }
        }
        // 2. Send unpause via IPC
        for sock in self.sockets.values() {
            let _ = ipc_set_property(sock, "pause", false);
        }
        self.is_paused = false;
    }

    pub fn stop_output(&mut self, output: &str) {
        let sock_path = socket_path_for_output(output);
        if ipc_is_alive(&sock_path) {
            let _ = send_ipc_command(&sock_path, r#"{"command": ["quit"]}"#);
        }
        if let Some(sock) = self.sockets.remove(output) {
            let _ = std::fs::remove_file(sock);
        }
        let _ = std::fs::remove_file(&sock_path);
        if let Some(mut child) = self.processes.remove(output) {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn stop_all(&mut self) {
        for (_, sock) in self.sockets.drain() {
            if ipc_is_alive(&sock) {
                let _ = send_ipc_command(&sock, r#"{"command": ["quit"]}"#);
            }
            let _ = std::fs::remove_file(sock);
        }
        for (_, mut child) in self.processes.drain() {
            let _ = child.kill();
            let _ = child.wait();
        }
        // Also terminate any running mpvpaper processes from other instances/PIDs
        let pids = find_mpvpaper_pids();
        for pid in pids {
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }
        // Clean up any stray sockets in socket_dir()
        let sock_dir = socket_dir();
        if let Ok(entries) = std::fs::read_dir(&sock_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("sock") {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        self.is_paused = false;
    }

    pub fn build_mpv_options(scaling: &str, mute: bool, volume: u8, hwdec: &str, is_image: bool) -> String {
        let mut opts = if is_image {
            String::from("image-display-duration=inf --loop-file=inf --pause=yes --no-config --no-audio --scale=spline36 --cscale=spline36 --dscale=mitchell --demuxer-max-bytes=8M --vd-lavc-threads=1")
        } else {
            let mut o = format!(
                "loop-file=inf --image-display-duration=inf --hwdec={} --no-config --demuxer-max-bytes=24M --demuxer-readahead-secs=2 --vd-lavc-threads=2 --background-color=#000000",
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
        assert!(opts_image.contains("--scale=spline36"));
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

    #[test]
    fn test_socket_paths() {
        let sock_default = socket_path_for_output("*");
        assert!(sock_default.to_string_lossy().ends_with("mpv_all.sock"));

        let sock_edp = socket_path_for_output("eDP-1");
        assert!(sock_edp.to_string_lossy().ends_with("mpv_eDP-1.sock"));

        let sock_special = socket_path_for_output("HDMI-A-1/test");
        assert!(sock_special.to_string_lossy().ends_with("mpv_HDMI-A-1test.sock"));
    }

    #[test]
    fn test_engine_init() {
        let engine = WallpaperEngine::new();
        assert!(!engine.is_paused);
        assert!(!engine.is_running());
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
    fn test_reap_and_processes_keys() {
        let mut engine = WallpaperEngine::new();
        assert!(engine.processes_keys().is_empty());
        let dead = engine.reap_dead_processes();
        assert!(dead.is_empty());
    }
}

