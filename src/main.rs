mod app;
mod cli;
mod config;
mod engine;
mod i18n;
mod scanner;
mod theme;
mod tray;
mod online;

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

fn main() -> cosmic::iced::Result {
    ensure_wayland_display();
    let args: Vec<String> = std::env::args().collect();
    if cli::handle_cli(&args) {
        return Ok(());
    }

    let hidden = args.iter().any(|a| a == "--hidden" || a == "--daemon" || a == "-d");
    let flags = app::AuraFlags { hidden };

    let settings = cosmic::app::Settings::default()
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(840.0)
                .min_height(580.0),
        )
        .exit_on_close(false);

    cosmic::app::run_single_instance::<app::AuraApp>(settings, flags)
}
