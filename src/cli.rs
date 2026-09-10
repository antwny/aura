use crate::config::Config;
use crate::engine::{detect_outputs, WallpaperEngine};
use crate::scanner::scan_directories;
use crate::theme::apply_cosmic_theme;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn is_pure_cli(args: &[String]) -> bool {
    if args.len() <= 1 {
        return false;
    }
    matches!(
        args[1].as_str(),
        "status" | "help" | "--help" | "-h" | "version" | "--version" | "-v" | "-V" | "check-update" | "update"
    )
}

pub fn handle_pure_cli(args: &[String]) {
    if args.len() <= 1 {
        return;
    }
    match args[1].as_str() {
        "status" => cmd_status(),
        "help" | "--help" | "-h" => print_help(),
        "version" | "--version" | "-v" | "-V" => {
            println!("Aura Live Wallpaper v{}", env!("CARGO_PKG_VERSION"));
        }
        "check-update" => cmd_check_update(),
        "update" => cmd_update(),
        _ => {}
    }
}

pub fn parse_flags(args: &[String]) -> (bool, Option<String>, Vec<String>) {
    if args.len() <= 1 {
        return (false, None, Vec::new());
    }

    match args[1].as_str() {
        "gui" => (false, None, Vec::new()),
        "--hidden" | "--daemon" | "-d" => (true, None, Vec::new()),
        "next" => (true, Some("next".into()), Vec::new()),
        "prev" => (true, Some("prev".into()), Vec::new()),
        "stop" => (true, Some("stop".into()), Vec::new()),
        "toggle-pause" | "pause" => (true, Some("toggle-pause".into()), Vec::new()),
        "apply" => {
            if args.len() >= 3 {
                let target = resolve_path(&args[2]);
                if !target.exists() {
                    eprintln!("Error: El archivo '{}' no existe.", target.display());
                    std::process::exit(1);
                }
                (true, Some("apply".into()), vec![target.to_string_lossy().to_string()])
            } else {
                eprintln!("Uso: aura apply <ruta_al_video_o_imagen>");
                std::process::exit(1);
            }
        }
        other => {
            let p = Path::new(other);
            if p.exists() && p.is_file() {
                let target = resolve_path(other);
                (true, Some("apply".into()), vec![target.to_string_lossy().to_string()])
            } else {
                eprintln!("Comando desconocido: '{}'. Ejecuta 'aura help' para ver opciones.", other);
                std::process::exit(1);
            }
        }
    }
}

fn resolve_path(p_str: &str) -> PathBuf {
    let p = PathBuf::from(p_str);
    if p.is_relative() {
        std::env::current_dir().unwrap_or_default().join(&p)
    } else {
        p
    }
}

#[allow(dead_code)]
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
    if let Ok(pid) = engine.set_wallpaper(&output, &path_str, &scaling, config.mute, config.volume, &config.hwdec) {
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

#[allow(dead_code)]
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
    if let Ok(pid) = engine.set_wallpaper(&output, &path_str, &scaling, config.mute, config.volume, &config.hwdec) {
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

#[allow(dead_code)]
fn cmd_stop() {
    let mut engine = WallpaperEngine::new();
    engine.stop_all();
    let mut config = Config::load();
    config.wallpapers.clear();
    config.current = None;
    let _ = config.save();
    println!("⏹ Aura: Fondo animado detenido.");
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

    if let Ok(pid) = engine.set_wallpaper(&output, &path_str, &scaling, config.mute, config.volume, &config.hwdec) {
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

fn parse_semver(v: &str) -> (u32, u32, u32) {
    let clean = v.trim_start_matches('v').trim();
    let parts: Vec<u32> = clean
        .split('.')
        .filter_map(|p| p.parse::<u32>().ok())
        .collect();
    (
        parts.get(0).copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    )
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    parse_semver(latest) > parse_semver(current)
}

fn cmd_check_update() {
    println!("🔍 Buscando actualizaciones de Aura en GitHub...");
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error al inicializar runtime: {}", e);
            return;
        }
    };

    rt.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent("Aura-Updater")
            .build()
            .unwrap_or_default();

        match client.get("https://api.github.com/repos/antwny/aura/releases/latest").send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        let latest_tag = json["tag_name"].as_str().unwrap_or_default();
                        let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
                        let clean_latest = latest_tag.trim_start_matches('v');
                        let clean_current = current_version.trim_start_matches('v');

                        if is_newer_version(clean_latest, clean_current) {
                            println!("🚀 ¡Nueva versión disponible! {} -> {}", current_version, latest_tag);
                            if let Some(body) = json["body"].as_str() {
                                println!("\nNotas de la versión:\n{}", body);
                            }
                            println!("\nPara actualizar automáticamente ejecuta:\n  aura update");
                        } else {
                            println!("✅ Tienes la versión más reciente (v{}).", env!("CARGO_PKG_VERSION"));
                        }
                    } else {
                        eprintln!("Error al procesar la respuesta de GitHub.");
                    }
                } else if resp.status().as_u16() == 404 {
                    println!("Aún no hay lanzamientos públicos publicados en GitHub Releases.");
                    println!("Versión instalada: v{}", env!("CARGO_PKG_VERSION"));
                } else {
                    eprintln!("GitHub API respondió con código: {}", resp.status());
                }
            }
            Err(e) => {
                eprintln!("No se pudo conectar con GitHub: {}", e);
            }
        }
    });
}

fn cmd_update() {
    println!("🚀 Comprobando y actualizando Aura...");
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error al inicializar runtime: {}", e);
            return;
        }
    };

    rt.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent("Aura-Updater")
            .build()
            .unwrap_or_default();

        let resp = match client.get("https://api.github.com/repos/antwny/aura/releases/latest").send().await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error al conectar con GitHub: {}", e);
                return;
            }
        };

        if !resp.status().is_success() {
            eprintln!("No se pudo obtener información del último lanzamiento (código {}).", resp.status());
            return;
        }

        let json = match resp.json::<serde_json::Value>().await {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Error al leer respuesta de GitHub: {}", e);
                return;
            }
        };

        let latest_tag = json["tag_name"].as_str().unwrap_or_default();
        let clean_latest = latest_tag.trim_start_matches('v');
        let current_version = env!("CARGO_PKG_VERSION");

        if !is_newer_version(clean_latest, current_version) {
            println!("✅ Ya tienes la última versión instalada (v{}).", current_version);
            return;
        }

        println!("Descargando versión {} (actual: v{})...", latest_tag, current_version);

        let assets = json["assets"].as_array();
        let mut download_url = None;
        if let Some(assets_list) = assets {
            for asset in assets_list {
                let name = asset["name"].as_str().unwrap_or_default();
                if name.ends_with(".tar.gz") && name.contains("linux") {
                    download_url = asset["browser_download_url"].as_str().map(|s| s.to_string());
                    break;
                }
            }
        }

        if let Some(url) = download_url {
            println!("Descargando paquete desde: {}", url);
            match client.get(&url).send().await {
                Ok(pkg_resp) => {
                    if let Ok(bytes) = pkg_resp.bytes().await {
                        let tmp_tar = std::env::temp_dir().join("aura_update.tar.gz");
                        let tmp_extract = std::env::temp_dir().join("aura_update_extracted");
                        let _ = tokio::fs::write(&tmp_tar, &bytes).await;
                        let _ = tokio::fs::create_dir_all(&tmp_extract).await;

                        let tar_status = Command::new("tar")
                            .args(["-xzf", &tmp_tar.to_string_lossy(), "-C", &tmp_extract.to_string_lossy()])
                            .status();

                        if tar_status.is_ok() && tar_status.unwrap().success() {
                            let mut new_bin = tmp_extract.join("aura");
                            if !new_bin.exists() {
                                if let Ok(entries) = std::fs::read_dir(&tmp_extract) {
                                    for entry in entries.filter_map(|e| e.ok()) {
                                        let candidate = entry.path().join("aura");
                                        if candidate.exists() {
                                            new_bin = candidate;
                                            break;
                                        }
                                    }
                                }
                            }

                            if new_bin.exists() {
                                let home = std::env::var("HOME").unwrap_or_default();
                                let target_bin = PathBuf::from(&home).join(".local/bin/aura");
                                if let Some(parent) = target_bin.parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                let tmp_target = target_bin.with_extension("new");
                                let _ = std::fs::copy(&new_bin, &tmp_target);
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::PermissionsExt;
                                    let _ = std::fs::set_permissions(&tmp_target, std::fs::Permissions::from_mode(0o755));
                                }
                                if std::fs::rename(&tmp_target, &target_bin).is_ok() {
                                    println!("🎉 ¡Aura actualizada exitosamente a {}!", latest_tag);
                                    println!("Ubicación: {}", target_bin.display());
                                    let _ = tokio::fs::remove_file(&tmp_tar).await;
                                    let _ = tokio::fs::remove_dir_all(&tmp_extract).await;
                                    return;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error al descargar la actualización: {}", e);
                    return;
                }
            }
        }

        println!("No se encontró un archivo precompilado compatible en la versión {}.", latest_tag);
        println!("Visita https://github.com/antwny/aura/releases para descargar manualmente.");
    });
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
    println!("  aura check-update     Comprueba si hay una nueva versión en GitHub");
    println!("  aura update           Descarga e instala la última actualización");
    println!("  aura --version, -v    Muestra la versión de Aura");
    println!("  aura help             Muestra esta ayuda");
    println!("\nConsejo para COSMIC: Asigna 'aura next' a un atajo de teclado (ej. Super + W) en Ajustes.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_pure_cli() {
        assert_eq!(is_pure_cli(&["aura".into()]), false);
        assert_eq!(is_pure_cli(&["aura".into(), "gui".into()]), false);
        assert_eq!(is_pure_cli(&["aura".into(), "--hidden".into()]), false);
        assert_eq!(is_pure_cli(&["aura".into(), "next".into()]), false);
        assert_eq!(is_pure_cli(&["aura".into(), "help".into()]), true);
        assert_eq!(is_pure_cli(&["aura".into(), "--help".into()]), true);
        assert_eq!(is_pure_cli(&["aura".into(), "-h".into()]), true);
        assert_eq!(is_pure_cli(&["aura".into(), "version".into()]), true);
        assert_eq!(is_pure_cli(&["aura".into(), "-v".into()]), true);
        assert_eq!(is_pure_cli(&["aura".into(), "status".into()]), true);
        assert_eq!(is_pure_cli(&["aura".into(), "check-update".into()]), true);
        assert_eq!(is_pure_cli(&["aura".into(), "update".into()]), true);
    }

    #[test]
    fn test_parse_flags() {
        let (hidden, action, args) = parse_flags(&["aura".into()]);
        assert_eq!(hidden, false);
        assert_eq!(action, None);
        assert!(args.is_empty());

        let (hidden, action, _) = parse_flags(&["aura".into(), "--hidden".into()]);
        assert_eq!(hidden, true);
        assert_eq!(action, None);

        let (hidden, action, _) = parse_flags(&["aura".into(), "next".into()]);
        assert_eq!(hidden, true);
        assert_eq!(action, Some("next".into()));

        let (hidden, action, _) = parse_flags(&["aura".into(), "toggle-pause".into()]);
        assert_eq!(hidden, true);
        assert_eq!(action, Some("toggle-pause".into()));

        let (hidden, action, _) = parse_flags(&["aura".into(), "stop".into()]);
        assert_eq!(hidden, true);
        assert_eq!(action, Some("stop".into()));
    }

    #[test]
    fn test_is_newer_version() {
        assert_eq!(is_newer_version("1.1.1", "1.1.0"), true);
        assert_eq!(is_newer_version("v1.1.1", "v1.1.0"), true);
        assert_eq!(is_newer_version("1.2.0", "1.1.9"), true);
        assert_eq!(is_newer_version("2.0.0", "1.99.99"), true);
        assert_eq!(is_newer_version("1.1.0", "1.1.1"), false);
        assert_eq!(is_newer_version("1.1.0", "1.1.0"), false);
        assert_eq!(is_newer_version("v1.1.0", "v1.1.0"), false);
    }
}
