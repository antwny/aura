use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    Auto,
    Dark,
    Light,
    Manual,
}

impl Default for ThemeMode {
    fn default() -> Self {
        ThemeMode::Auto
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwitcherStyle {
    Classic,
    Cinematic,
    Honeycomb,
}

impl Default for SwitcherStyle {
    fn default() -> Self {
        SwitcherStyle::Honeycomb
    }
}

fn default_auto_dark() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub version: Option<String>,
    pub current: Option<String>,
    pub wallpapers: HashMap<String, String>,
    pub dirs: Vec<String>,
    pub custom_videos: Vec<String>,
    pub output: String,
    pub scaling: HashMap<String, String>,
    pub auto_theme: bool,
    #[serde(default = "default_auto_dark")]
    pub auto_dark: bool,
    #[serde(default)]
    pub theme_mode: ThemeMode,
    pub rotation: bool,
    pub interval: u64, // In minutes
    pub order: String,  // "random" | "sequential"
    pub seq_index: usize,
    pub mute: bool,
    #[serde(default = "default_volume")]
    pub volume: u8,
    pub hwdec: String, // "auto-safe" | "vaapi" | "nvdec" | "no"
    #[serde(default = "default_gpu_preference")]
    pub gpu_preference: String, // "auto" | "discrete" | "integrated" | render node
    pub smart_pause: bool,
    #[serde(default = "default_auto_pause")]
    pub auto_pause: bool,
    #[serde(default)]
    pub pause_on_battery: bool,
    pub keep_running_on_close: bool,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub favorites: Vec<String>,
    #[serde(default = "default_library_sort")]
    pub library_sort: String,
    #[serde(default)]
    pub rotation_only_favorites: bool,
    #[serde(default = "default_tray_click_action")]
    pub tray_click_action: String,
    #[serde(default)]
    pub switcher_only_favorites: bool,
    #[serde(default = "default_switcher_position")]
    pub switcher_position: String,
    #[serde(default)]
    pub switcher_style: SwitcherStyle,
}

fn default_switcher_position() -> String {
    "top".to_string()
}

fn default_tray_click_action() -> String {
    "switcher".to_string()
}

fn default_library_sort() -> String {
    "newest".to_string()
}

fn default_auto_pause() -> bool {
    true
}

fn default_volume() -> u8 {
    100
}

fn default_gpu_preference() -> String {
    "auto".to_string()
}

fn default_language() -> String {
    crate::i18n::detect_system_language().code().to_string()
}

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let clean = version.trim().trim_start_matches('v');
    let mut parts = clean.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

fn migrate_config(cfg: &mut Config) -> bool {
    let old_version = cfg.version.as_deref().and_then(parse_version);
    let current_version = parse_version(CURRENT_VERSION).unwrap_or((0, 0, 0));
    let mut changed = false;

    if let Some(old) = old_version {
        if old < (1, 6, 0) && cfg.switcher_style != SwitcherStyle::Honeycomb {
            cfg.switcher_style = SwitcherStyle::Honeycomb;
            changed = true;
        }
        if old < current_version {
            cfg.version = Some(CURRENT_VERSION.into());
            changed = true;
        }
    } else {
        // Missing or malformed version: stamp it without discarding user choices.
        cfg.version = Some(CURRENT_VERSION.into());
        changed = true;
    }

    changed
}

pub fn config_home() -> PathBuf {
    #[cfg(test)]
    crate::test_support::init();

    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config")
}

fn resolve_xdg_user_dir(name: &str) -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        let user_dirs_file = config_home().join("user-dirs.dirs");
        if let Ok(content) = std::fs::read_to_string(user_dirs_file) {
            let key = format!("XDG_{}_DIR=", name.to_uppercase());
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with(&key) {
                    let val = trimmed.trim_start_matches(&key).trim_matches('"');
                    let expanded = val.replace("$HOME", &home);
                    let p = PathBuf::from(expanded);
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }
    }

    if let Ok(output) = std::process::Command::new("xdg-user-dir").arg(name).output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                let p = PathBuf::from(path_str);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    None
}

pub fn default_library_dirs() -> Vec<String> {
    let mut dirs = Vec::new();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());

    // 1. Online downloaded wallpapers dir (always first)
    let online_dir = crate::online::wallpapers_online_dir().to_string_lossy().to_string();
    dirs.push(online_dir);

    // 2. Specific wallpaper subdirectories only (never general Downloads or root Videos/Pictures)
    if let Some(p) = resolve_xdg_user_dir("PICTURES") {
        for sub in &["Wallpapers", "Wallpaper", "Fondos"] {
            let wall_sub = p.join(sub);
            if wall_sub.exists() {
                let s = wall_sub.to_string_lossy().to_string();
                if !dirs.contains(&s) { dirs.push(s); }
            }
        }
    }
    if let Some(v) = resolve_xdg_user_dir("VIDEOS") {
        for sub in &["Wallpapers", "Wallpaper", "Fondos"] {
            let wall_sub = v.join(sub);
            if wall_sub.exists() {
                let s = wall_sub.to_string_lossy().to_string();
                if !dirs.contains(&s) { dirs.push(s); }
            }
        }
    }

    // 3. Fallback dedicated wallpaper candidates (multilingual & common paths)
    let candidates = [
        format!("{}/Pictures/Wallpapers", home),
        format!("{}/Pictures/Wallpaper", home),
        format!("{}/Imágenes/Wallpapers", home),
        format!("{}/Videos/Wallpapers", home),
        format!("{}/Videos/Wallpaper", home),
        format!("{}/Vídeos/Wallpapers", home),
        format!("{}/Wallpapers", home),
        format!("{}/Fondos", home),
    ];

    for c in candidates {
        if std::path::Path::new(&c).exists() && !dirs.contains(&c) {
            dirs.push(c);
        }
    }

    // If only online_dir was added, provide standard pictures/wallpapers path
    if dirs.len() <= 1 {
        dirs.push(format!("{}/Pictures/Wallpapers", home));
    }

    dirs
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: Some(CURRENT_VERSION.into()),
            current: None,
            wallpapers: HashMap::new(),
            dirs: default_library_dirs(),
            custom_videos: Vec::new(),
            output: "*".into(),
            scaling: HashMap::new(),
            auto_theme: true,
            auto_dark: true,
            theme_mode: ThemeMode::Auto,
            rotation: false,
            interval: 30,
            order: "random".into(),
            seq_index: 0,
            mute: true,
            volume: 100,
            hwdec: "auto-safe".into(),
            gpu_preference: default_gpu_preference(),
            smart_pause: true,
            auto_pause: true,
            pause_on_battery: false,
            keep_running_on_close: true,
            autostart: false,
            language: default_language(),
            favorites: Vec::new(),
            library_sort: default_library_sort(),
            rotation_only_favorites: false,
            tray_click_action: default_tray_click_action(),
            switcher_only_favorites: false,
            switcher_position: default_switcher_position(),
            switcher_style: SwitcherStyle::Honeycomb,
        }
    }
}

impl Config {
    /// Records `path` as active on `output`, keeping the map in step with the engine:
    /// "*" replaces every per-output entry, and a per-output apply ends the wildcard.
    /// GNOME has a single desktop background, so there everything maps to "*".
    pub fn record_wallpaper(&mut self, output: &str, path: &str) {
        if output == "*" || crate::engine::gnome::is_active() {
            self.wallpapers.clear();
            self.wallpapers.insert("*".into(), path.into());
        } else {
            self.wallpapers.remove("*");
            self.wallpapers.insert(output.into(), path.into());
        }
        self.current = Some(path.into());
    }

    pub fn config_dir() -> PathBuf {
        config_home().join("aura")
    }

    pub fn config_file() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    fn backup_file() -> PathBuf {
        Self::config_dir().join("config.json.bak")
    }

    fn load_from_path(path: &Path) -> Option<(Config, serde_json::Value)> {
        let content = std::fs::read_to_string(path).ok()?;
        match serde_json::from_str::<Config>(&content) {
            Ok(cfg) => {
                let value = serde_json::from_str::<serde_json::Value>(&content).ok()?;
                Some((cfg, value))
            }
            Err(e) => {
                eprintln!("[Aura Config] No se pudo interpretar {}: {}", path.display(), e);
                None
            }
        }
    }

    fn quarantine(path: &Path) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("config.json");
        let dest = path.with_file_name(format!("{}.corrupt.{}", name, stamp));
        if std::fs::rename(path, &dest).is_ok() {
            eprintln!(
                "[Aura Config] Configuración corrupta guardada en {}",
                dest.display()
            );
        }
    }

    pub fn load() -> Self {
        let path = Self::config_file();
        let mut should_save = false;

        let loaded = match Self::load_from_path(&path) {
            Some(loaded) => Some(loaded),
            None => {
                if path.exists() {
                    Self::quarantine(&path);
                    should_save = true;
                }
                Self::load_from_path(&Self::backup_file()).map(|loaded| {
                    should_save = true;
                    loaded
                })
            }
        };

        let Some((mut cfg, val)) = loaded else {
            let cfg = Self::default();
            let _ = cfg.save();
            return cfg;
        };

        let mut changed = should_save || migrate_config(&mut cfg);

        if val.get("theme_mode").is_none() {
            if let Some(false) = val.get("auto_dark").and_then(|v| v.as_bool()) {
                cfg.theme_mode = ThemeMode::Manual;
                changed = true;
            }
        }

        let online_dir = crate::online::wallpapers_online_dir().to_string_lossy().to_string();
        if !cfg.dirs.contains(&online_dir) {
            cfg.dirs.insert(0, online_dir);
            changed = true;
        }

        let dirs_before = cfg.dirs.len();
        cfg.dirs.retain(|d| {
            let p = std::path::Path::new(d);
            let is_downloads = p
                .file_name()
                .and_then(|n| n.to_str())
                .map(|name| {
                    name.eq_ignore_ascii_case("downloads")
                        || name.eq_ignore_ascii_case("descargas")
                })
                .unwrap_or(false);
            !is_downloads
        });
        changed |= cfg.dirs.len() != dirs_before;

        let has_valid_dir = cfg.dirs.iter().any(|d| std::path::Path::new(d).exists());
        if !has_valid_dir {
            for d in default_library_dirs() {
                if !cfg.dirs.contains(&d) {
                    cfg.dirs.push(d);
                    changed = true;
                }
            }
        }

        let old_interval = cfg.interval;
        if cfg.interval == 0 {
            cfg.interval = 30;
        } else {
            cfg.interval = cfg.interval.clamp(1, 1440);
        }
        changed |= cfg.interval != old_interval;

        if changed {
            let _ = cfg.save();
        }
        cfg
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        self.save_to(&Self::config_file())
    }

    fn save_to(&self, target: &Path) -> Result<(), std::io::Error> {
        let dir = target.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(dir)?;

        let file_name = target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("config.json");
        let backup = dir.join(format!("{}.bak", file_name));
        if target.exists() {
            let _ = std::fs::copy(target, &backup);
        }

        let content = serde_json::to_string_pretty(self)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tmp = dir.join(format!(
            "{}.tmp.{}.{}",
            file_name,
            std::process::id(),
            stamp
        ));

        let result = (|| -> Result<(), std::io::Error> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp)?;
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&tmp, target)?;
            if let Ok(dir_handle) = std::fs::File::open(dir) {
                let _ = dir_handle.sync_all();
            }
            Ok(())
        })();

        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        result
    }

    pub fn is_favorite(&self, path: &str) -> bool {
        self.favorites.iter().any(|f| f == path)
    }

    pub fn toggle_favorite(&mut self, path: &str) -> bool {
        if let Some(pos) = self.favorites.iter().position(|f| f == path) {
            self.favorites.remove(pos);
            false
        } else {
            self.favorites.push(path.to_string());
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.order, "random");
        assert_eq!(cfg.interval, 30);
        assert!(cfg.auto_theme);
        assert!(cfg.auto_dark);
        assert_eq!(cfg.theme_mode, ThemeMode::Auto);
        assert!(cfg.mute);
        assert!(cfg.smart_pause);
        assert!(cfg.keep_running_on_close);
        assert_eq!(cfg.volume, 100);
        assert!(!cfg.pause_on_battery);
        assert_eq!(cfg.hwdec, "auto-safe");
        assert_eq!(cfg.gpu_preference, "auto");
        assert_eq!(cfg.output, "*");
        assert!(!cfg.dirs.is_empty());
        assert!(!cfg.dirs.iter().any(|d| d.to_lowercase().ends_with("downloads") || d.to_lowercase().ends_with("descargas")));
        assert!(!cfg.autostart);
        assert!(!cfg.rotation_only_favorites);
    }

    #[test]
    fn test_record_wallpaper_keeps_wildcard_exclusive() {
        if crate::engine::gnome::is_active() {
            return;
        }
        let mut cfg = Config::default();
        cfg.record_wallpaper("DP-1", "/a.mp4");
        cfg.record_wallpaper("HDMI-A-1", "/b.mp4");
        assert_eq!(cfg.wallpapers.len(), 2);

        cfg.record_wallpaper("*", "/c.mp4");
        assert_eq!(cfg.wallpapers.len(), 1);
        assert_eq!(cfg.wallpapers.get("*").unwrap(), "/c.mp4");

        cfg.record_wallpaper("DP-1", "/d.mp4");
        assert!(!cfg.wallpapers.contains_key("*"));
        assert_eq!(cfg.wallpapers.get("DP-1").unwrap(), "/d.mp4");
        assert_eq!(cfg.current.as_deref(), Some("/d.mp4"));
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let mut cfg = Config::default();
        cfg.wallpapers.insert("DP-1".into(), "/path/to/vid.mp4".into());
        cfg.scaling.insert("DP-1".into(), "fill".into());
        cfg.current = Some("/path/to/vid.mp4".into());
        cfg.language = "en".into();
        cfg.rotation_only_favorites = true;

        let json = serde_json::to_string_pretty(&cfg).expect("serialize config");
        let deserialized: Config = serde_json::from_str(&json).expect("deserialize config");

        assert_eq!(deserialized.current, Some("/path/to/vid.mp4".into()));
        assert_eq!(deserialized.wallpapers.get("DP-1").unwrap(), "/path/to/vid.mp4");
        assert_eq!(deserialized.scaling.get("DP-1").unwrap(), "fill");
        assert_eq!(deserialized.language, "en");
        assert!(deserialized.rotation_only_favorites);
    }

    #[test]
    fn test_config_backward_compatibility_missing_language() {
        // Old configs without the "language" key should deserialize properly
        let old_json = r#"{
            "current": null,
            "wallpapers": {},
            "dirs": ["/home/test/Videos"],
            "custom_videos": [],
            "output": "*",
            "scaling": {},
            "auto_theme": true,
            "auto_dark": true,
            "rotation": false,
            "interval": 30,
            "order": "random",
            "seq_index": 0,
            "mute": true,
            "hwdec": "auto-safe",
            "smart_pause": true,
            "keep_running_on_close": true
        }"#;

        let cfg: Result<Config, _> = serde_json::from_str(old_json);
        assert!(cfg.is_ok());
        let cfg = cfg.unwrap();
        assert!(!cfg.language.is_empty());
        assert!(cfg.auto_pause);
        assert_eq!(cfg.theme_mode, ThemeMode::Auto);
        assert_eq!(cfg.gpu_preference, "auto");
    }

    #[test]
    fn test_theme_mode_serialization() {
        for mode in [ThemeMode::Auto, ThemeMode::Dark, ThemeMode::Light, ThemeMode::Manual] {
            let mut cfg = Config::default();
            cfg.theme_mode = mode;
            let json = serde_json::to_string(&cfg).expect("serialize");
            let deserialized: Config = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(deserialized.theme_mode, mode);
        }
    }

    #[test]
    fn test_config_paths() {
        let dir = Config::config_dir();
        let file = Config::config_file();
        assert!(dir.ends_with(".config/aura"));
        assert!(file.ends_with(".config/aura/config.json"));
    }

    #[test]
    fn test_favorites_and_sort() {
        let mut cfg = Config::default();
        assert_eq!(cfg.library_sort, "newest");
        assert!(!cfg.is_favorite("/path/to/test.mp4"));

        assert!(cfg.toggle_favorite("/path/to/test.mp4"));
        assert!(cfg.is_favorite("/path/to/test.mp4"));

        assert!(!cfg.toggle_favorite("/path/to/test.mp4"));
        assert!(!cfg.is_favorite("/path/to/test.mp4"));

        assert!(!cfg.rotation_only_favorites);
        cfg.rotation_only_favorites = true;
        assert!(cfg.rotation_only_favorites);
    }

    #[test]
    fn test_rotation_only_favorites_pool() {
        let mut cfg = Config::default();
        let paths = vec![
            "/wallpapers/1.mp4".to_string(),
            "/wallpapers/2.mp4".to_string(),
            "/wallpapers/3.mp4".to_string(),
        ];

        // 1. When rotation_only_favorites is false: pool includes all
        let pool: Vec<&String> = if cfg.rotation_only_favorites {
            let favs: Vec<&String> = paths.iter().filter(|p| cfg.is_favorite(p)).collect();
            if favs.is_empty() { paths.iter().collect() } else { favs }
        } else {
            paths.iter().collect()
        };
        assert_eq!(pool.len(), 3);

        // 2. When rotation_only_favorites is true, but no favorites: falls back to all
        cfg.rotation_only_favorites = true;
        let pool: Vec<&String> = if cfg.rotation_only_favorites {
            let favs: Vec<&String> = paths.iter().filter(|p| cfg.is_favorite(p)).collect();
            if favs.is_empty() { paths.iter().collect() } else { favs }
        } else {
            paths.iter().collect()
        };
        assert_eq!(pool.len(), 3);

        // 3. When favorites are added: filters strictly to favorites
        cfg.toggle_favorite("/wallpapers/2.mp4");
        assert!(cfg.is_favorite("/wallpapers/2.mp4"));
        let pool: Vec<&String> = if cfg.rotation_only_favorites {
            let favs: Vec<&String> = paths.iter().filter(|p| cfg.is_favorite(p)).collect();
            if favs.is_empty() { paths.iter().collect() } else { favs }
        } else {
            paths.iter().collect()
        };
        assert_eq!(pool.len(), 1);
        assert_eq!(pool[0], "/wallpapers/2.mp4");
    }

    #[test]
    fn test_switcher_config() {
        let mut cfg = Config::default();
        assert_eq!(cfg.tray_click_action, "switcher");
        assert!(!cfg.switcher_only_favorites);
        assert_eq!(cfg.switcher_position, "top");
        assert_eq!(cfg.switcher_style, SwitcherStyle::Honeycomb);

        cfg.tray_click_action = "main_window".to_string();
        cfg.switcher_only_favorites = true;
        cfg.switcher_position = "left".to_string();
        cfg.switcher_style = SwitcherStyle::Classic;
        assert_eq!(cfg.tray_click_action, "main_window");
        assert!(cfg.switcher_only_favorites);
        assert_eq!(cfg.switcher_position, "left");
        assert_eq!(cfg.switcher_style, SwitcherStyle::Classic);

        let json = serde_json::to_string(&cfg).expect("serialize");
        let deserialized: Config = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.switcher_style, SwitcherStyle::Classic);

        cfg.switcher_style = SwitcherStyle::Honeycomb;
        let json_honeycomb = serde_json::to_string(&cfg).expect("serialize honeycomb");
        assert!(json_honeycomb.contains("\"switcher_style\":\"honeycomb\""));
        let deserialized_honeycomb: Config = serde_json::from_str(&json_honeycomb).expect("deserialize honeycomb");
        assert_eq!(deserialized_honeycomb.switcher_style, SwitcherStyle::Honeycomb);
    }

    #[test]
    fn test_custom_interval_serialization_and_clamping() {
        let mut cfg = Config::default();
        cfg.interval = 45;
        let json = serde_json::to_string_pretty(&cfg).expect("serialize config");
        let mut deserialized: Config = serde_json::from_str(&json).expect("deserialize config");
        assert_eq!(deserialized.interval, 45);

        // Test clamping on corrupted zero or excessive values
        deserialized.interval = 0;
        if deserialized.interval == 0 {
            deserialized.interval = 30;
        } else {
            deserialized.interval = deserialized.interval.clamp(1, 1440);
        }
        assert_eq!(deserialized.interval, 30);

        deserialized.interval = 5000;
        deserialized.interval = deserialized.interval.clamp(1, 1440);
        assert_eq!(deserialized.interval, 1440);

        deserialized.interval = 7;
        deserialized.interval = deserialized.interval.clamp(1, 1440);
        assert_eq!(deserialized.interval, 7);
    }

    #[test]
    fn test_legacy_config_migration_preserves_user_choices() {
        // Simulate a legacy config without "version" field and with an explicit switcher style.
        let legacy_json = r#"{
            "current": "/wallpapers/live.mp4",
            "wallpapers": { "eDP-1": "/wallpapers/live.mp4" },
            "dirs": ["/home/user/Wallpapers"],
            "custom_videos": [],
            "output": "eDP-1",
            "scaling": {},
            "auto_theme": true,
            "auto_dark": true,
            "theme_mode": "dark",
            "rotation": true,
            "interval": 15,
            "order": "random",
            "seq_index": 0,
            "mute": false,
            "volume": 80,
            "hwdec": "auto-safe",
            "gpu_preference": "auto",
            "smart_pause": true,
            "auto_pause": true,
            "pause_on_battery": true,
            "keep_running_on_close": true,
            "autostart": false,
            "language": "es",
            "favorites": ["/wallpapers/live.mp4"],
            "library_sort": "newest",
            "rotation_only_favorites": true,
            "tray_click_action": "switcher",
            "switcher_only_favorites": false,
            "switcher_position": "top",
            "switcher_style": "cinematic"
        }"#;

        let mut cfg: Config = serde_json::from_str(legacy_json).expect("deserialize legacy json");
        assert_eq!(cfg.version, None);
        assert_eq!(cfg.switcher_style, SwitcherStyle::Cinematic);
        assert_eq!(cfg.interval, 15);
        assert_eq!(cfg.volume, 80);
        assert_eq!(cfg.theme_mode, ThemeMode::Dark);
        assert!(cfg.rotation_only_favorites);

        assert!(migrate_config(&mut cfg));

        assert_eq!(cfg.interval, 15);
        assert_eq!(cfg.volume, 80);
        assert_eq!(cfg.theme_mode, ThemeMode::Dark);
        assert!(cfg.rotation_only_favorites);
        assert_eq!(cfg.favorites.len(), 1);
        assert_eq!(cfg.output, "eDP-1");
        assert_eq!(cfg.switcher_style, SwitcherStyle::Cinematic);
        assert_eq!(cfg.version.as_deref(), Some(CURRENT_VERSION));
    }

    #[test]
    fn test_v171_config_does_not_reset_switcher_style() {
        let mut cfg = Config::default();
        cfg.version = Some("1.7.0".into());
        cfg.switcher_style = SwitcherStyle::Cinematic;

        assert!(migrate_config(&mut cfg));
        assert_eq!(cfg.switcher_style, SwitcherStyle::Cinematic);
        assert_eq!(cfg.version.as_deref(), Some(CURRENT_VERSION));
    }

    #[test]
    fn test_future_config_version_is_not_downgraded() {
        let mut cfg = Config::default();
        cfg.version = Some("99.0.0".into());
        cfg.switcher_style = SwitcherStyle::Classic;

        assert!(!migrate_config(&mut cfg));
        assert_eq!(cfg.version.as_deref(), Some("99.0.0"));
        assert_eq!(cfg.switcher_style, SwitcherStyle::Classic);
    }

    #[test]
    fn test_save_creates_backup_and_loads_it_on_corruption() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.json");
        let mut cfg = Config::default();
        cfg.volume = 42;
        cfg.save_to(&path).expect("save config");
        cfg.save_to(&path).expect("save config again");
        assert!(path.exists());

        std::fs::write(&path, "{ not valid json").expect("corrupt config");
        assert!(Config::load_from_path(&path).is_none());
        let backup = dir.path().join("config.json.bak");
        assert!(backup.exists());
        let (restored, _) = Config::load_from_path(&backup).expect("load backup");
        assert_eq!(restored.volume, 42);
    }
}
