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
}

impl Default for Config {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let default_dirs = vec![
            format!("{}/Wallpapers/Aura", home),
            format!("{}/Wallpapers/Papyrus", home),
            format!("{}/Wallpapers", home),
            format!("{}/Videos", home),
            format!("{}/Downloads", home),
        ];

        Self {
            current: None,
            wallpapers: HashMap::new(),
            dirs: default_dirs,
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
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
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
                if let Ok(cfg) = serde_json::from_str::<Config>(&content) {
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
