use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

/// Determines the user's standard Pictures / Imágenes directory via XDG or standard paths.
pub fn user_pictures_dir() -> PathBuf {
    // 1. Try xdg-user-dir PICTURES
    if let Ok(output) = std::process::Command::new("xdg-user-dir").arg("PICTURES").output() {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                let p = PathBuf::from(s);
                if p.is_absolute() {
                    return p;
                }
            }
        }
    }

    // 2. Try parsing ~/.config/user-dirs.dirs
    if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(&home);
        let user_dirs = home_path.join(".config/user-dirs.dirs");
        if let Ok(content) = std::fs::read_to_string(user_dirs) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("XDG_PICTURES_DIR=") {
                    let val = trimmed.trim_start_matches("XDG_PICTURES_DIR=").trim_matches('"');
                    let expanded = val.replace("$HOME", &home);
                    return PathBuf::from(expanded);
                }
            }
        }

        // 3. Fallbacks: Spanish 'Imágenes' or English 'Pictures'
        let p_esp = home_path.join("Imágenes");
        if p_esp.exists() {
            return p_esp;
        }
        let p_eng = home_path.join("Pictures");
        if p_eng.exists() {
            return p_eng;
        }
        return home_path.join("Pictures");
    }

    PathBuf::from("/tmp")
}

pub fn wallpapers_online_dir() -> PathBuf {
    static CACHED: OnceLock<PathBuf> = OnceLock::new();
    CACHED.get_or_init(|| {
        let pics = user_pictures_dir();
        let dir = pics.join("Wallpapers/Aura");
        let _ = std::fs::create_dir_all(&dir);

        // One-time automatic migration of any existing wallpapers from legacy hidden folder
        let legacy_base = if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                PathBuf::from(xdg)
            } else {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
                PathBuf::from(home).join(".local/share")
            }
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".local/share")
        };
        let legacy_dir = legacy_base.join("aura/wallpapers/online");
        if legacy_dir.exists() && legacy_dir != dir {
            if let Ok(entries) = std::fs::read_dir(&legacy_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let src = entry.path();
                    if src.is_file() {
                        let dst = dir.join(entry.file_name());
                        if !dst.exists() {
                            let _ = std::fs::copy(&src, &dst);
                        }
                    }
                }
            }
        }

        dir
    }).clone()
}

pub fn thumbs_online_cache_dir() -> PathBuf {
    static CACHED: OnceLock<PathBuf> = OnceLock::new();
    CACHED.get_or_init(|| {
        let base = if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
            if !xdg.is_empty() {
                PathBuf::from(xdg)
            } else {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
                PathBuf::from(home).join(".cache")
            }
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".cache")
        };
        let dir = base.join("aura/online_thumbs");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }).clone()
}

pub fn clean_filename(prefix: &str, id: &str, ext: &str) -> String {
    let clean_id: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    format!("{}_{}.{}", prefix, clean_id, ext)
}

pub fn is_downloaded(prefix: &str, id: &str) -> Option<PathBuf> {
    let dir = wallpapers_online_dir();
    let extensions = ["jpg", "jpeg", "png", "webp"];
    for ext in &extensions {
        let candidate = dir.join(clean_filename(prefix, id, ext));
        if candidate.exists() {
            if let Ok(metadata) = candidate.metadata() {
                if metadata.len() > 1024 {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

pub async fn download_to_file(client: &reqwest::Client, url: &str, destination: &Path) -> Result<PathBuf, String> {
    if let Some(parent) = destination.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let response = client
        .get(url)
        .timeout(Duration::from_secs(25))
        .header("User-Agent", concat!("Aura-LiveWallpaper-Client/", env!("CARGO_PKG_VERSION")))
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned HTTP error {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if bytes.is_empty() {
        return Err("Received empty file from server".into());
    }

    let tmp_path = destination.with_extension("tmp");
    tokio::fs::write(&tmp_path, &bytes)
        .await
        .map_err(|e| format!("Failed to write temporary file: {}", e))?;

    tokio::fs::rename(&tmp_path, destination)
        .await
        .map_err(|e| format!("Failed to finalize downloaded file: {}", e))?;

    Ok(destination.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_filename() {
        let name = clean_filename("bing", "OHR.MountFuji_EN-US123", "jpg");
        assert_eq!(name, "bing_OHR_MountFuji_EN-US123.jpg");

        let name_symbols = clean_filename("wallhaven", "x9k!@#$%-test", "png");
        assert_eq!(name_symbols, "wallhaven_x9k_____-test.png");
    }

    #[test]
    fn test_directories_non_empty() {
        let wall_dir = wallpapers_online_dir();
        let thumb_dir = thumbs_online_cache_dir();
        assert!(wall_dir.to_string_lossy().contains("Wallpapers/Aura") || wall_dir.to_string_lossy().contains("aura"));
        assert!(thumb_dir.to_string_lossy().contains("online_thumbs"));
    }
}

