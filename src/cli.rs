use crate::config::Config;
use crate::engine::{detect_outputs, DesktopEnvironment, WallpaperEngine};
use crate::scanner::scan_directories;
use crate::theme::apply_cosmic_theme;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn is_pure_cli(args: &[String]) -> bool {
    if args.len() <= 1 {
        return false;
    }
    matches!(
        args[1].as_str(),
        "status" | "help" | "--help" | "-h" | "version" | "--version" | "-v" | "-V" | "check-update" | "update" | "mode" | "theme-mode"
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
            println!("{}{}", cli_language().cli_version(), env!("CARGO_PKG_VERSION"));
        }
        "check-update" => cmd_check_update(),
        "update" => cmd_update(),
        "mode" | "theme-mode" => cmd_theme_mode(args),
        _ => {}
    }
}

/// Language used by the terminal, matching the saved GUI language. Kept in one
/// place so `aura status`, `--help` and errors never disagree with Settings.
fn cli_language() -> crate::i18n::Language {
    if let Ok(value) = std::env::var("AURA_LANG") {
        if let Some(language) = crate::i18n::parse_language(&value) {
            return language;
        }
    }
    crate::i18n::Language::from_code(&Config::load().language)
}

pub fn parse_flags(args: &[String]) -> (bool, Option<String>, Vec<String>) {
    if args.len() <= 1 {
        return (false, None, Vec::new());
    }

    match args[1].as_str() {
        "gui" => (false, None, Vec::new()),
        "--hidden" | "--daemon" | "-d" => (true, None, Vec::new()),
        "switcher" | "hud" | "menu" => (true, Some("switcher".into()), Vec::new()),
        "next" => (true, Some("next".into()), Vec::new()),
        "prev" => (true, Some("prev".into()), Vec::new()),
        "stop" => (true, Some("stop".into()), Vec::new()),
        "toggle-pause" | "pause" => (true, Some("toggle-pause".into()), Vec::new()),
        "apply" => {
            if args.len() >= 3 {
                let lang = cli_language();
                let target = resolve_path(&args[2]);
                if !target.exists() {
                    eprintln!("{}", lang.cli_error_file_missing(&target.display().to_string()));
                    std::process::exit(1);
                }
                if !target.is_file() {
                    eprintln!("{}", lang.cli_error_not_regular(&target.display().to_string()));
                    std::process::exit(1);
                }
                let ext = target.extension().and_then(|e| e.to_str()).unwrap_or_default();
                if !crate::scanner::is_supported_wallpaper_ext(ext) {
                    eprintln!("{}", lang.cli_error_unsupported_format(ext));
                    std::process::exit(1);
                }
                (true, Some("apply".into()), vec![target.to_string_lossy().to_string()])
            } else {
                eprintln!("{}", cli_language().cli_usage_apply());
                std::process::exit(1);
            }
        }
        other => {
            let lang = cli_language();
            let p = Path::new(other);
            if p.exists() && p.is_file() {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or_default();
                if crate::scanner::is_supported_wallpaper_ext(ext) {
                    let target = resolve_path(other);
                    (true, Some("apply".into()), vec![target.to_string_lossy().to_string()])
                } else {
                    eprintln!("{}", lang.cli_error_unknown_command(other));
                    std::process::exit(1);
                }
            } else {
                eprintln!("{}", lang.cli_error_unknown_command(other));
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

/// Drops the shared wildcard player when every connected output already has
/// its own player, avoiding a hidden duplicate renderer.
fn retire_redundant_wildcard(engine: &mut WallpaperEngine) {
    let outputs: Vec<String> = detect_outputs()
        .iter()
        .map(|output| output.name.clone())
        .collect();
    engine.retire_redundant_wildcard(&outputs);
}

#[allow(dead_code)]
fn cmd_next() {
    let mut config = Config::load();
    let lang = cli_language();
    let videos = scan_directories(&config.dirs, &config.custom_videos);
    if videos.is_empty() {
        println!("{}", lang.cli_no_wallpapers());
        return;
    }

    let favorites: HashSet<String> = videos
        .iter()
        .map(|item| item.path.to_string_lossy().to_string())
        .filter(|path| config.is_favorite(path))
        .collect();
    let pool = crate::scanner::rotation_pool_refs(
        &videos,
        &favorites,
        config.rotation_only_favorites,
        config.rotation_media_filter,
    );
    if pool.is_empty() {
        println!("{}", lang.cli_no_rotation_match());
        return;
    }

    let next_idx = if config.order == "random" {
        (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as usize) % pool.len()
    } else {
        (config.seq_index + 1) % pool.len()
    };

    config.seq_index = next_idx;
    let video = pool[next_idx];
    let output = if config.output.is_empty() { "*".into() } else { config.output.clone() };
    let scaling = config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());

    let mut engine = WallpaperEngine::new();
    let path_str = video.path.to_string_lossy().to_string();
    if let Ok(pid) = engine.set_wallpaper(&output, &path_str, &scaling, config.mute, config.volume, &config.hwdec, config.auto_pause, &config.gpu_preference) {
        config.record_wallpaper(&output, &path_str);
        let _ = config.save();
        retire_redundant_wildcard(&mut engine);

        if config.auto_theme || config.theme_mode != crate::config::ThemeMode::Manual {
            if let Some(thumb) = &video.thumb_path {
                apply_cosmic_theme(
                    thumb,
                    config.auto_theme,
                    config.theme_mode,
                    DesktopEnvironment::detect() == DesktopEnvironment::Cosmic,
                );
            }
        }

        println!("{}", lang.cli_wallpaper_changed(&video.name, pid));
    } else {
        eprintln!("{}", lang.cli_engine_error());
    }
}

#[allow(dead_code)]
fn cmd_prev() {
    let mut config = Config::load();
    let lang = cli_language();
    let videos = scan_directories(&config.dirs, &config.custom_videos);
    if videos.is_empty() {
        println!("{}", lang.cli_no_wallpapers());
        return;
    }

    let favorites: HashSet<String> = videos
        .iter()
        .map(|item| item.path.to_string_lossy().to_string())
        .filter(|path| config.is_favorite(path))
        .collect();
    let pool = crate::scanner::rotation_pool_refs(
        &videos,
        &favorites,
        config.rotation_only_favorites,
        config.rotation_media_filter,
    );
    if pool.is_empty() {
        println!("{}", lang.cli_no_rotation_match());
        return;
    }

    let prev_idx = if config.seq_index == 0 || config.seq_index >= pool.len() {
        pool.len() - 1
    } else {
        config.seq_index - 1
    };

    config.seq_index = prev_idx;
    let video = pool[prev_idx];
    let output = if config.output.is_empty() { "*".into() } else { config.output.clone() };
    let scaling = config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());

    let mut engine = WallpaperEngine::new();
    let path_str = video.path.to_string_lossy().to_string();
    if let Ok(pid) = engine.set_wallpaper(&output, &path_str, &scaling, config.mute, config.volume, &config.hwdec, config.auto_pause, &config.gpu_preference) {
        config.record_wallpaper(&output, &path_str);
        let _ = config.save();
        retire_redundant_wildcard(&mut engine);

        if config.auto_theme || config.theme_mode != crate::config::ThemeMode::Manual {
            if let Some(thumb) = &video.thumb_path {
                apply_cosmic_theme(
                    thumb,
                    config.auto_theme,
                    config.theme_mode,
                    DesktopEnvironment::detect() == DesktopEnvironment::Cosmic,
                );
            }
        }

        println!("{}", lang.cli_previous_wallpaper(&video.name, pid));
    }
}

#[allow(dead_code)]
fn cmd_stop() {
    let mut engine = WallpaperEngine::new();
    engine.stop_all();
    let mut config = Config::load();
    let lang = cli_language();
    config.wallpapers.clear();
    config.current = None;
    let _ = config.save();
    println!("{}", lang.cli_stopped());
}

#[allow(dead_code)]
fn cmd_toggle_pause() {
    let lang = cli_language();
    let pids = crate::engine::mpvpaper::find_mpvpaper_pids();
    if pids.is_empty() {
        println!("{}", lang.cli_no_mpvpaper());
        return;
    }

    let mut engine = WallpaperEngine::new();
    let is_paused = engine.toggle_pause();
    if is_paused {
        println!("{}", lang.cli_paused());
    } else {
        println!("{}", lang.cli_resumed());
    }
}

#[allow(dead_code)]
fn cmd_apply(path_arg: &str) {
    let lang = cli_language();
    let p = PathBuf::from(path_arg);
    let full_path = if p.is_relative() {
        std::env::current_dir().unwrap_or_default().join(&p)
    } else {
        p
    };

    if !full_path.exists() {
        eprintln!("{}", lang.cli_error_file_missing(&full_path.display().to_string()));
        return;
    }

    if !full_path.is_file() {
        eprintln!("{}", lang.cli_error_not_regular(&full_path.display().to_string()));
        return;
    }

    let ext = full_path.extension().and_then(|e| e.to_str()).unwrap_or_default();
    if !crate::scanner::is_supported_wallpaper_ext(ext) {
        eprintln!("{}", lang.cli_error_unsupported_format(ext));
        return;
    }

    let mut config = Config::load();
    let outputs = detect_outputs();
    let output = outputs.first().map(|o| o.name.clone()).unwrap_or_else(|| "*".into());
    let scaling = config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());

    let mut engine = WallpaperEngine::new();
    let path_str = full_path.to_string_lossy().to_string();

    match engine.set_wallpaper(&output, &path_str, &scaling, config.mute, config.volume, &config.hwdec, config.auto_pause, &config.gpu_preference) {
        Ok(pid) => {
            config.record_wallpaper(&output, &path_str);
            if !config.custom_videos.contains(&path_str) {
                config.custom_videos.push(path_str.clone());
            }
            let _ = config.save();
            retire_redundant_wildcard(&mut engine);

            let thumb = crate::scanner::thumbs::thumb_path_for_video(&full_path);
            if thumb.exists() && (config.auto_theme || config.theme_mode != crate::config::ThemeMode::Manual) {
                apply_cosmic_theme(
                    &thumb,
                    config.auto_theme,
                    config.theme_mode,
                    DesktopEnvironment::detect() == DesktopEnvironment::Cosmic,
                );
            }

            println!("{}", lang.cli_applied(pid, &full_path.display().to_string()));
        }
        Err(e) => {
            eprintln!("{}", lang.cli_apply_failed(&e.to_string()));
        }
    }
}

fn cmd_status() {
    let config = Config::load();
    let lang = cli_language();
    let l = lang.cli_labels();
    println!("{}", l.status_title);
    println!("──────────────────────────────────────────");

    let backend = crate::engine::Backend::detect(config.backend);
    let label = |text: &str| format!("{:<23}", text);
    println!(
        "{}{}",
        label(l.desktop),
        crate::engine::DesktopEnvironment::detect().pretty_name()
    );
    println!("{}{}", label(l.backend), backend.label());
    if backend == crate::engine::Backend::Kde {
        println!(
            "{}{}",
            label(l.dbus),
            if crate::engine::kde::dbus_available() {
                l.available
            } else {
                l.missing
            }
        );
        println!(
            "{}{}",
            label(l.plugin),
            if crate::engine::kde::plugin_installed() {
                l.plugin_detected
            } else {
                l.plugin_missing
            }
        );
    }

    // Runtime engine health check
    let health = crate::engine::mpvpaper::check_engine_health();
    match &health {
        crate::engine::mpvpaper::EngineHealth::Ready => {
            println!("{}{}", label(l.engine), l.engine_ready);
        }
        crate::engine::mpvpaper::EngineHealth::MissingBinary => {
            let (distro, cmd) = crate::engine::mpvpaper::detect_distro_command();
            println!("{}{}", label(l.engine), l.engine_missing_binary);
            println!("{}{} {}: {}", " ".repeat(23), l.install_hint, distro, cmd);
        }
        crate::engine::mpvpaper::EngineHealth::MissingLibrary(_) => {
            let (distro, cmd) = crate::engine::mpvpaper::detect_distro_command();
            println!("{}{}", label(l.engine), l.engine_missing_library);
            println!("{}{} {}: {}", " ".repeat(23), l.install_hint, distro, cmd);
        }
        crate::engine::mpvpaper::EngineHealth::ExecutionError(err) => {
            println!(
                "{}{}: {}",
                label(l.engine),
                l.engine_error,
                err
            );
        }
    }

    // Check actual running processes
    let pids = crate::engine::mpvpaper::find_mpvpaper_pids();
    if !pids.is_empty() {
        let paused_count = pids
            .iter()
            .filter(|&&pid| crate::engine::mpvpaper::is_pid_stopped(pid))
            .count();
        let ipc_paused = crate::engine::mpvpaper::all_mpvpaper_paused_by_ipc();
        if paused_count == pids.len() || ipc_paused == Some(true) {
            println!(
                "{}{} (PIDs: {:?})",
                label(l.run_state),
                l.paused,
                pids
            );
        } else {
            println!(
                "{}{} (PIDs: {:?})",
                label(l.run_state),
                l.playing,
                pids
            );
        }
    } else {
        println!("{}{}", label(l.run_state), l.stopped);
    }

    if let Some(curr) = &config.current {
        println!("{}{}", label(l.wallpaper), curr);
    } else {
        println!("{}{}", label(l.wallpaper), l.none);
    }

    println!("{}", l.displays);
    for (out, wall) in &config.wallpapers {
        let sc = config
            .scaling
            .get(out)
            .map(|s| s.as_str())
            .unwrap_or("fit");
        println!("  • {}: {} ({}: {})", out, wall, l.scaling, sc);
    }

    let pool_str = if config.rotation_only_favorites {
        format!("{} [{}]", l.pool_favorites, config.favorites.len())
    } else {
        l.pool_all.to_string()
    };
    let media_str = match config.rotation_media_filter {
        crate::config::RotationMediaFilter::All => l.media_all,
        crate::config::RotationMediaFilter::Live => l.media_live,
        crate::config::RotationMediaFilter::Static => l.media_static,
    };
    println!(
        "{}{}",
        label(l.rotation),
        lang.cli_rotation_line(
            config.rotation,
            &pool_str,
            media_str,
            config.order == "random",
            config.interval,
        )
    );
    println!(
        "{}{}",
        label(l.auto_theme),
        if config.auto_theme {
            l.enabled
        } else {
            l.disabled
        }
    );
    println!(
        "{}{}",
        label(l.appearance_mode),
        lang.cli_theme_mode_name(config.theme_mode)
    );
    println!(
        "{}{}",
        label(l.auto_pause),
        format!(
            "{} ({})",
            if config.auto_pause {
                l.enabled
            } else {
                l.disabled
            },
            lang.auto_pause_mode_name(config.auto_pause_mode)
        )
    );
    println!(
        "{}{}",
        label(l.muted),
        if config.mute { l.yes } else { l.no }
    );
    println!("{}{}%", label(l.volume), config.volume);
    println!("{}{}", label(l.gpu_accel), config.hwdec);
    println!("{}{}", label(l.gpu_pref), config.gpu_preference);
    println!(
        "{}{}",
        label(l.autostart),
        if config.autostart || WallpaperEngine::is_autostart_enabled() {
            l.enabled
        } else {
            l.disabled
        }
    );
}

fn cmd_check_update() {
    let lang = cli_language();
    println!("{}", lang.cli_update_checking());
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", lang.cli_runtime_error(&e.to_string()));
            return;
        }
    };

    rt.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent("Aura-Updater")
            .build()
            .unwrap_or_default();

        match crate::online::updater::check_latest_release(&client).await {
            Ok(release) => {
                let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
                let clean_latest = release.tag.trim_start_matches('v');
                let clean_current = current_version.trim_start_matches('v');

                if crate::online::updater::is_newer_version(clean_latest, clean_current) {
                    println!(
                        "{}",
                        lang.cli_update_available(&current_version, &release.tag)
                    );
                    if !release.body.trim().is_empty() {
                        println!("{}", lang.cli_update_notes(&release.body));
                    }
                    println!("{}", lang.cli_update_instructions());
                } else {
                    println!(
                        "{}",
                        lang.cli_update_up_to_date(env!("CARGO_PKG_VERSION"))
                    );
                }
            }
            Err(e) => {
                eprintln!("{}", lang.cli_update_check_failed(&e.to_string()));
            }
        }
    });
}

fn cmd_update() {
    let lang = cli_language();
    println!("{}", lang.cli_update_starting());
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", lang.cli_runtime_error(&e.to_string()));
            return;
        }
    };

    rt.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent("Aura-Updater")
            .build()
            .unwrap_or_default();

        let release = match crate::online::updater::check_latest_release(&client).await {
            Ok(res) => res,
            Err(e) => {
                eprintln!("{}", lang.cli_update_query_failed(&e.to_string()));
                return;
            }
        };

        let clean_latest = release.tag.trim_start_matches('v');
        let current_version = env!("CARGO_PKG_VERSION");

        if !crate::online::updater::is_newer_version(clean_latest, current_version) {
            println!("{}", lang.cli_update_up_to_date(current_version));
            return;
        }

        let (url, checksum_url) = match (release.download_url, release.checksum_url) {
            (Some(url), Some(checksum_url)) => (url, checksum_url),
            _ => {
                println!("{}", lang.cli_update_no_package(&release.tag));
                println!("{}", lang.cli_update_manual());
                return;
            }
        };

        println!(
            "{}",
            lang.cli_update_downloading(&release.tag, current_version)
        );
        match crate::online::updater::perform_update(&client, &url, &checksum_url).await {
            Ok(dest) => {
                println!("{}", lang.cli_update_success(&release.tag));
                println!("{}", lang.cli_update_location(&dest));
            }
            Err(e) => {
                eprintln!("{}", lang.cli_update_failed(&e.to_string()));
            }
        }
    });
}

fn cmd_theme_mode(args: &[String]) {
    let lang = cli_language();
    if args.len() < 3 {
        let config = Config::load();
        println!(
            "{}",
            lang.cli_theme_current(lang.cli_theme_mode_name(config.theme_mode))
        );
        println!("{}", lang.cli_theme_usage());
        return;
    }

    let target_mode = match args[2].to_lowercase().as_str() {
        "dark" | "oscuro" => crate::config::ThemeMode::Dark,
        "light" | "claro" => crate::config::ThemeMode::Light,
        "auto" | "dynamic" | "automatico" | "automático" => crate::config::ThemeMode::Auto,
        "manual" | "system" | "sistema" => crate::config::ThemeMode::Manual,
        other => {
            eprintln!("{}", lang.cli_theme_unknown(other));
            std::process::exit(1);
        }
    };

    let mut config = Config::load();
    config.theme_mode = target_mode;
    config.auto_dark = target_mode == crate::config::ThemeMode::Auto;
    let _ = config.save();

    match target_mode {
        crate::config::ThemeMode::Dark => crate::theme::apply_cosmic_mode(true),
        crate::config::ThemeMode::Light => crate::theme::apply_cosmic_mode(false),
        crate::config::ThemeMode::Auto => {
            if let Some(cur) = &config.current {
                let p = PathBuf::from(cur);
                let thumb = crate::scanner::thumbs::thumb_path_for_video(&p);
                if thumb.exists() {
                    apply_cosmic_theme(
                        &thumb,
                        config.auto_theme,
                        target_mode,
                        DesktopEnvironment::detect() == DesktopEnvironment::Cosmic,
                    );
                } else if crate::scanner::is_supported_wallpaper_ext(
                    p.extension().and_then(|e| e.to_str()).unwrap_or_default(),
                ) {
                    apply_cosmic_theme(
                        &p,
                        config.auto_theme,
                        target_mode,
                        DesktopEnvironment::detect() == DesktopEnvironment::Cosmic,
                    );
                }
            }
        }
        crate::config::ThemeMode::Manual => {}
    }

    println!(
        "{}",
        lang.cli_theme_changed(lang.cli_theme_mode_name(target_mode))
    );
}

fn print_help() {
    println!("{}", cli_language().cli_help());
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
        assert_eq!(is_pure_cli(&["aura".into(), "mode".into()]), true);
        assert_eq!(is_pure_cli(&["aura".into(), "theme-mode".into()]), true);
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

        let (hidden, action, _) = parse_flags(&["aura".into(), "switcher".into()]);
        assert_eq!(hidden, true);
        assert_eq!(action, Some("switcher".into()));

        let (hidden, action, _) = parse_flags(&["aura".into(), "stop".into()]);
        assert_eq!(hidden, true);
        assert_eq!(action, Some("stop".into()));
    }

    #[test]
    fn test_is_newer_version() {
        use crate::online::updater::is_newer_version;
        assert_eq!(is_newer_version("1.1.1", "1.1.0"), true);
        assert_eq!(is_newer_version("v1.1.1", "v1.1.0"), true);
        assert_eq!(is_newer_version("1.2.0", "1.1.9"), true);
        assert_eq!(is_newer_version("2.0.0", "1.99.99"), true);
        assert_eq!(is_newer_version("1.1.0", "1.1.1"), false);
        assert_eq!(is_newer_version("1.1.0", "1.1.0"), false);
        assert_eq!(is_newer_version("v1.1.0", "v1.1.0"), false);
    }
}
