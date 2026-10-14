mod app;
mod cli;
mod config;
mod engine;
mod i18n;
mod scanner;
mod theme;
mod tray;
mod online;
mod system;
mod ui;

fn ensure_wayland_display() {
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            for name in &["wayland-1", "wayland-0", "wayland-2"] {
                let p = std::path::Path::new(&runtime_dir).join(name);
                if p.exists() {
                    std::env::set_var("WAYLAND_DISPLAY", name);
                    return;
                }
            }
            if let Ok(entries) = std::fs::read_dir(&runtime_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let fname = entry.file_name();
                    let s = fname.to_string_lossy();
                    if s.starts_with("wayland-") && !s.ends_with(".lock") {
                        std::env::set_var("WAYLAND_DISPLAY", s.as_ref());
                        return;
                    }
                }
            }
        }
    }
}

fn ensure_environment() {
    ensure_wayland_display();
    // Ensure user and local bin directories are in PATH so helper utilities (mpvpaper, ffmpeg) are always found
    if let Ok(home) = std::env::var("HOME") {
        let local_bin = format!("{}/.local/bin", home);
        if let Ok(path) = std::env::var("PATH") {
            if !path.split(':').any(|p| p == local_bin) {
                std::env::set_var("PATH", format!("{}:{}", local_bin, path));
            }
        } else {
            std::env::set_var("PATH", local_bin);
        }
    }
}

/// Returns true if the "Cosmic" icon theme is installed in any XDG icon directory.
fn cosmic_icon_theme_installed() -> bool {
    let mut dirs = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let home = std::path::PathBuf::from(home);
        dirs.push(home.join(".icons"));
        match std::env::var_os("XDG_DATA_HOME") {
            Some(data_home) => dirs.push(std::path::PathBuf::from(data_home).join("icons")),
            None => dirs.push(home.join(".local/share/icons")),
        }
    }
    let data_dirs = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_string());
    dirs.extend(data_dirs.split(':').map(|d| std::path::Path::new(d).join("icons")));
    dirs.iter().any(|d| d.join(cosmic::icon_theme::COSMIC).join("index.theme").is_file())
}

fn main() -> cosmic::iced::Result {
    ensure_environment();
    let args: Vec<String> = std::env::args().collect();
    if cli::is_pure_cli(&args) {
        cli::handle_pure_cli(&args);
        return Ok(());
    }

    let (hidden, action, action_args) = cli::parse_flags(&args);
    if let Some(ref act) = action {
        println!("🌌 Aura: Procesando acción '{}'...", act);
    }
    let flags = app::AuraFlags {
        hidden,
        action,
        args: action_args,
    };

    let mut settings = cosmic::app::Settings::default()
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(840.0)
                .min_height(580.0),
        )
        .exit_on_close(false);

    // Outside COSMIC the default "Cosmic" icon theme is usually missing, and the
    // hicolor fallback has no window-*-symbolic icons, so the header bar's
    // window controls render blank. Adwaita ships with GTK on most systems.
    if !cosmic_icon_theme_installed() {
        settings = settings.default_icon_theme("Adwaita");
    }

    if hidden {
        settings = settings.no_main_window(true);
    }

    cosmic::app::run_single_instance::<app::AuraApp>(settings, flags)
}
