use crate::config::Config;
use crate::engine::{detect_outputs, WallpaperEngine};
use crate::scanner::scan_directories;
use crate::theme::apply_cosmic_theme;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn handle_cli(args: &[String]) -> bool {
    if args.len() <= 1 {
        return false; // No arguments, launch GUI
    }

    match args[1].as_str() {
        "next" => {
            cmd_next();
            true
        }
        "prev" => {
            cmd_prev();
            true
        }
        "stop" => {
            cmd_stop();
            true
        }
        "toggle-pause" | "pause" => {
            cmd_toggle_pause();
            true
        }
        "apply" => {
            if args.len() >= 3 {
                cmd_apply(&args[2]);
            } else {
                eprintln!("Uso: aura apply <ruta_al_video>");
            }
            true
        }
        "status" => {
            cmd_status();
            true
        }
        "help" | "--help" | "-h" => {
            print_help();
            true
        }
        "version" | "--version" | "-v" | "-V" => {
            println!("Aura Live Wallpaper v{}", env!("CARGO_PKG_VERSION"));
            true
        }
        "gui" | "--hidden" | "--daemon" | "-d" => false, // Explicit GUI or background launch
        other => {
            // If passed a video path directly: `aura video.mp4`
            let p = Path::new(other);
            if p.exists() && p.is_file() {
                cmd_apply(other);
                true
            } else {
                eprintln!("Comando desconocido: '{}'. Ejecuta 'aura help' para ver opciones.", other);
                true
            }
        }
    }
}

fn cmd_next() {
    let mut config = Config::load();
    let videos = scan_directories(&config.dirs, &config.custom_videos);
    if videos.is_empty() {
        println!("No se encontraron fondos de pantalla configurados.");
        return;
    }

    let next_idx = if config.order == "random" {
        (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as usize) % videos.len()
    } else {
        (config.seq_index + 1) % videos.len()
    };

    config.seq_index = next_idx;
    let video = &videos[next_idx];
    let output = if config.output.is_empty() { "*".into() } else { config.output.clone() };
    let scaling = config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());

    let mut engine = WallpaperEngine::new();
    let path_str = video.path.to_string_lossy().to_string();
    if let Ok(pid) = engine.set_wallpaper(&output, &path_str, &scaling, config.mute, &config.hwdec) {
        config.wallpapers.insert(output.clone(), path_str.clone());
        config.current = Some(path_str.clone());
        let _ = config.save();

        if config.auto_theme {
            if let Some(thumb) = &video.thumb_path {
                apply_cosmic_theme(thumb, config.auto_dark);
            }
        }

        println!("✨ Aura: Fondo cambiado a '{}' (PID: {})", video.name, pid);
    } else {
        eprintln!("Error al cambiar el fondo con mpvpaper.");
    }
}

fn cmd_prev() {
    let mut config = Config::load();
    let videos = scan_directories(&config.dirs, &config.custom_videos);
    if videos.is_empty() {
        println!("No se encontraron fondos de pantalla configurados.");
        return;
    }

    let prev_idx = if config.seq_index == 0 {
        videos.len() - 1
    } else {
        config.seq_index - 1
    };

    config.seq_index = prev_idx;
    let video = &videos[prev_idx];
    let output = if config.output.is_empty() { "*".into() } else { config.output.clone() };
    let scaling = config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());

    let mut engine = WallpaperEngine::new();
    let path_str = video.path.to_string_lossy().to_string();
    if let Ok(pid) = engine.set_wallpaper(&output, &path_str, &scaling, config.mute, &config.hwdec) {
        config.wallpapers.insert(output.clone(), path_str.clone());
        config.current = Some(path_str.clone());
        let _ = config.save();

        if config.auto_theme {
            if let Some(thumb) = &video.thumb_path {
                apply_cosmic_theme(thumb, config.auto_dark);
            }
        }

        println!("✨ Aura: Fondo anterior activado: '{}' (PID: {})", video.name, pid);
    }
}

fn cmd_stop() {
    let mut engine = WallpaperEngine::new();
    engine.stop_all();
    let mut config = Config::load();
    config.wallpapers.clear();
    config.current = None;
    let _ = config.save();
    println!("⏹ Aura: Fondo animado detenido.");
}

fn cmd_toggle_pause() {
    // Check if mpvpaper is running
    let status = Command::new("pkill").args(["-0", "-x", "mpvpaper"]).status();
    if status.is_err() || !status.unwrap().success() {
        println!("No hay ningún fondo de mpvpaper ejecutándose.");
        return;
    }

    let pkill_stop = Command::new("pkill").args(["-STOP", "-x", "mpvpaper"]).status();
    if pkill_stop.is_ok() && pkill_stop.unwrap().success() {
        println!("⏸ Aura: Fondo pausado (0% GPU/CPU).");
    } else {
        let _ = Command::new("pkill").args(["-CONT", "-x", "mpvpaper"]).status();
        println!("▶ Aura: Fondo reanudado.");
    }
}

fn cmd_apply(path_arg: &str) {
    let p = PathBuf::from(path_arg);
    let full_path = if p.is_relative() {
        std::env::current_dir().unwrap_or_default().join(&p)
    } else {
        p
    };

    if !full_path.exists() {
        eprintln!("Error: El archivo '{}' no existe.", full_path.display());
        return;
    }

    let mut config = Config::load();
    let outputs = detect_outputs();
    let output = outputs.first().map(|o| o.name.clone()).unwrap_or_else(|| "*".into());
    let scaling = config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());

    let mut engine = WallpaperEngine::new();
    let path_str = full_path.to_string_lossy().to_string();

    if let Ok(pid) = engine.set_wallpaper(&output, &path_str, &scaling, config.mute, &config.hwdec) {
        config.wallpapers.insert(output.clone(), path_str.clone());
        config.current = Some(path_str.clone());
        if !config.custom_videos.contains(&path_str) {
            config.custom_videos.push(path_str.clone());
        }
        let _ = config.save();

        let thumb = crate::scanner::thumbs::thumb_path_for_video(&full_path);
        if thumb.exists() && config.auto_theme {
            apply_cosmic_theme(&thumb, config.auto_dark);
        }

        println!("✨ Aura: Fondo aplicado exitosamente (PID: {}) -> {}", pid, full_path.display());
    } else {
        eprintln!("Error: No se pudo iniciar mpvpaper con el video seleccionado.");
    }
}

fn cmd_status() {
    let config = Config::load();
    println!("🌌 Aura Live Wallpaper — Estado del Sistema");
    println!("──────────────────────────────────────────");
    if let Some(curr) = &config.current {
        println!("Fondo activo: {}", curr);
    } else {
        println!("Fondo activo: Ninguno");
    }
    println!("Pantallas configuradas:");
    for (out, wall) in &config.wallpapers {
        let sc = config.scaling.get(out).map(|s| s.as_str()).unwrap_or("fit");
        println!("  • {}: {} (Escala: {})", out, wall, sc);
    }
    println!("Auto-Tema COSMIC: {}", if config.auto_theme { "Activado" } else { "Desactivado" });
    println!("Silenciado: {}", if config.mute { "Sí" } else { "No" });
    println!("Aceleración GPU: {}", config.hwdec);
    println!("Inicio automático: {}", if WallpaperEngine::is_autostart_enabled() { "Activado" } else { "Desactivado" });
}

fn print_help() {
    println!("🌌 Aura — Gestor Nativo de Fondos Animados para COSMIC (Pop!_OS)");
    println!("\nUso:");
    println!("  aura                  Abre la interfaz gráfica de COSMIC");
    println!("  aura next             Cambia al siguiente fondo de pantalla");
    println!("  aura prev             Regresa al fondo anterior");
    println!("  aura stop             Detiene la reproducción de fondos");
    println!("  aura toggle-pause     Pausa o reanuda la reproducción (0% GPU al pausar)");
    println!("  aura apply <archivo>  Aplica inmediatamente un archivo de video");
    println!("  aura status           Muestra información del fondo y pantallas");
    println!("  aura --version, -v    Muestra la versión de Aura");
    println!("  aura help             Muestra esta ayuda");
    println!("\nConsejo para COSMIC: Asigna 'aura next' a un atajo de teclado (ej. Super + W) en Ajustes.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_cli_no_args_opens_gui() {
        let args = vec!["aura".to_string()];
        assert_eq!(handle_cli(&args), false);
    }

    #[test]
    fn test_handle_cli_gui_flag() {
        let args = vec!["aura".to_string(), "gui".to_string()];
        assert_eq!(handle_cli(&args), false);
    }

    #[test]
    fn test_handle_cli_daemon_flag() {
        assert_eq!(handle_cli(&["aura".into(), "--hidden".into()]), false);
        assert_eq!(handle_cli(&["aura".into(), "--daemon".into()]), false);
        assert_eq!(handle_cli(&["aura".into(), "-d".into()]), false);
    }

    #[test]
    fn test_handle_cli_help() {
        let args = vec!["aura".to_string(), "help".to_string()];
        assert_eq!(handle_cli(&args), true);

        let args = vec!["aura".to_string(), "--help".to_string()];
        assert_eq!(handle_cli(&args), true);

        let args = vec!["aura".to_string(), "-h".to_string()];
        assert_eq!(handle_cli(&args), true);
    }

    #[test]
    fn test_handle_cli_version() {
        let args = vec!["aura".to_string(), "--version".to_string()];
        assert_eq!(handle_cli(&args), true);

        let args = vec!["aura".to_string(), "-v".to_string()];
        assert_eq!(handle_cli(&args), true);

        let args = vec!["aura".to_string(), "version".to_string()];
        assert_eq!(handle_cli(&args), true);
    }
}
