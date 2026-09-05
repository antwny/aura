use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub current: Option<String>,
    pub wallpapers: HashMap<String, String>,
    pub dirs: Vec<String>,
    pub custom_videos: Vec<String>,
    pub output: String,
    pub scaling: HashMap<String, String>,
    pub auto_theme: bool,
    pub auto_dark: bool,
    pub rotation: bool,
    pub interval: u64, // In minutes
    pub order: String,  // "random" | "sequential"
    pub seq_index: usize,
    pub mute: bool,
    pub hwdec: String, // "auto-safe" | "vaapi" | "nvdec" | "no"
    pub smart_pause: bool,
    pub keep_running_on_close: bool,
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String {
    crate::i18n::detect_system_language().code().to_string()
}

fn resolve_xdg_user_dir(name: &str) -> Option<PathBuf> {
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

    // 2. Localized XDG user directories (Videos, Pictures, Download)
    if let Some(v) = resolve_xdg_user_dir("VIDEOS") {
        let s = v.to_string_lossy().to_string();
        if !dirs.contains(&s) { dirs.push(s); }
    }
    if let Some(p) = resolve_xdg_user_dir("PICTURES") {
        let s = p.to_string_lossy().to_string();
        if !dirs.contains(&s) { dirs.push(s); }
        let wall_sub = p.join("Wallpapers");
        if wall_sub.exists() {
            let s = wall_sub.to_string_lossy().to_string();
            if !dirs.contains(&s) { dirs.push(s); }
        }
    }
    if let Some(d) = resolve_xdg_user_dir("DOWNLOAD") {
        let s = d.to_string_lossy().to_string();
        if !dirs.contains(&s) { dirs.push(s); }
    }

    // 3. Fallback standard candidates (multilingual & common paths)
    let candidates = [
        format!("{}/Wallpapers/Aura", home),
        format!("{}/Wallpapers", home),
        format!("{}/Fondos", home),
        format!("{}/Vídeos", home),
        format!("{}/Videos", home),
        format!("{}/Imágenes", home),
        format!("{}/Pictures", home),
        format!("{}/Descargas", home),
        format!("{}/Downloads", home),
    ];

    for c in candidates {
        if std::path::Path::new(&c).exists() && !dirs.contains(&c) {
            dirs.push(c);
        }
    }

    // If only online_dir is found, keep common paths so user sees where to place files
    if dirs.len() <= 1 {
        dirs.push(format!("{}/Videos", home));
        dirs.push(format!("{}/Wallpapers", home));
        dirs.push(format!("{}/Downloads", home));
    }

    dirs
}

impl Default for Config {
    fn default() -> Self {
        Self {
            current: None,
            wallpapers: HashMap::new(),
            dirs: default_library_dirs(),
            custom_videos: Vec::new(),
            output: "*".into(),
            scaling: HashMap::new(),
            auto_theme: true,
            auto_dark: true,
            rotation: false,
            interval: 30,
            order: "random".into(),
            seq_index: 0,
            mute: true,
            hwdec: "auto-safe".into(),
            smart_pause: true,
            keep_running_on_close: true,
            language: default_language(),
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg).join("aura");
            }
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(".config").join("aura")
    }

    pub fn config_file() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_file();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(mut cfg) = serde_json::from_str::<Config>(&content) {
                    let online_dir = crate::online::wallpapers_online_dir().to_string_lossy().to_string();
                    if !cfg.dirs.contains(&online_dir) {
                        cfg.dirs.insert(0, online_dir);
                    }
                    // If all dirs in config no longer exist, refresh with default library dirs
                    let has_valid_dir = cfg.dirs.iter().any(|d| std::path::Path::new(d).exists());
                    if !has_valid_dir {
                        for d in default_library_dirs() {
                            if !cfg.dirs.contains(&d) {
                                cfg.dirs.push(d);
                            }
                        }
                    }
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(Self::config_file(), content)?;
        Ok(())
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
        assert!(cfg.mute);
        assert!(cfg.smart_pause);
        assert!(cfg.keep_running_on_close);
        assert_eq!(cfg.hwdec, "auto-safe");
        assert_eq!(cfg.output, "*");
        assert!(!cfg.dirs.is_empty());
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let mut cfg = Config::default();
        cfg.wallpapers.insert("DP-1".into(), "/path/to/vid.mp4".into());
        cfg.scaling.insert("DP-1".into(), "fill".into());
        cfg.current = Some("/path/to/vid.mp4".into());
        cfg.language = "en".into();

        let json = serde_json::to_string_pretty(&cfg).expect("serialize config");
        let deserialized: Config = serde_json::from_str(&json).expect("deserialize config");

        assert_eq!(deserialized.current, Some("/path/to/vid.mp4".into()));
        assert_eq!(deserialized.wallpapers.get("DP-1").unwrap(), "/path/to/vid.mp4");
        assert_eq!(deserialized.scaling.get("DP-1").unwrap(), "fill");
        assert_eq!(deserialized.language, "en");
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
    }

    #[test]
    fn test_config_paths() {
        let dir = Config::config_dir();
        let file = Config::config_file();
        assert!(dir.ends_with(".config/aura"));
        assert!(file.ends_with(".config/aura/config.json"));
    }
}
