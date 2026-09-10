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
                        let _ = std::fs::remove_file(&src);
                    }
                }
            }
        }

        migrate_legacy_bing_wallpapers(&dir);

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

/// Sanitizes a wallpaper title to be safe for filenames across Linux, NTFS, and FAT filesystems
/// while preserving clean, natural, human-readable words in any language.
pub fn clean_title_filename(title: &str) -> String {
    let mut cleaned = String::with_capacity(title.len());
    for c in title.chars() {
        match c {
            '/' | '\\' | ':' | '|' => cleaned.push('-'),
            '*' | '?' | '"' | '<' | '>' => cleaned.push(' '),
            c if c.is_control() => cleaned.push(' '),
            c => cleaned.push(c),
        }
    }

    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let result = words.join(" ");
    let trimmed = result.trim_matches(|c: char| c == '.' || c == '-' || c == '_' || c == ' ');

    if trimmed.is_empty() {
        "Bing Wallpaper".to_string()
    } else if trimmed.chars().count() > 80 {
        let truncated: String = trimmed.chars().take(80).collect();
        truncated.trim_end_matches(|c: char| c == '.' || c == '-' || c == '_' || c == ' ').to_string()
    } else {
        trimmed.to_string()
    }
}

/// Automatically migrates legacy downloaded Bing wallpapers (e.g. `bing_adfdb411e...jpg`)
/// in the user's wallpapers directory to friendly human-readable titles (e.g. `Fields of gold.jpg`).
pub fn migrate_legacy_bing_wallpapers(dir: &Path) {
    if !dir.exists() {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let cache_dir = thumbs_online_cache_dir();
    let mut hash_map = std::collections::HashMap::new();

    let try_paths = [
        cache_dir.join("bing_daily.json"),
        cache_dir.join("bing_archive.json"),
    ];

    for p in &try_paths {
        if let Ok(content) = std::fs::read_to_string(p) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                let list_opt = val["images"].as_array().or_else(|| val["data"].as_array());
                if let Some(list) = list_opt {
                    for item in list {
                        let title = item["title"].as_str().unwrap_or_default().trim();
                        if title.is_empty() {
                            continue;
                        }
                        if let Some(hsh) = item["hsh"].as_str() {
                            if !hsh.is_empty() {
                                hash_map.insert(hsh.to_string(), title.to_string());
                            }
                        }
                        if let Some(urlbase) = item["urlbase"].as_str() {
                            let clean_url = urlbase.trim_start_matches("/th?id=");
                            hash_map.insert(clean_url.to_string(), title.to_string());
                        }
                    }
                }
            }
        }
    }

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = match path.file_name().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => continue,
        };

        if !file_name.starts_with("bing_") {
            continue;
        }

        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("jpg");
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        let raw_id = stem.trim_start_matches("bing_");

        let resolved_title = hash_map.get(raw_id).cloned().or_else(|| {
            if raw_id.contains("OHR") {
                let part = raw_id.split('.').nth(1).unwrap_or(raw_id);
                let core = part.split('_').next().unwrap_or(part);
                if !core.is_empty() {
                    let mut s = String::new();
                    let mut prev_is_lower = false;
                    for c in core.chars() {
                        if prev_is_lower && c.is_uppercase() {
                            s.push(' ');
                        }
                        prev_is_lower = c.is_lowercase();
                        s.push(c);
                    }
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
            None
        });

        if let Some(title) = resolved_title {
            let clean = clean_title_filename(&title);
            let target_name = format!("{}.{}", clean, ext);
            let target_path = dir.join(&target_name);
            if !target_path.exists() {
                if std::fs::rename(&path, &target_path).is_ok() {
                    let old_thumb = crate::scanner::thumbs::thumb_path_for_video(&path);
                    let new_thumb = crate::scanner::thumbs::thumb_path_for_video(&target_path);
                    if old_thumb.exists() && !new_thumb.exists() {
                        let _ = std::fs::rename(&old_thumb, &new_thumb);
                    }
                }
            } else {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

pub fn clean_filename(prefix: &str, id: &str, ext: &str) -> String {
    let clean_id: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    format!("{}_{}.{}", prefix, clean_id, ext)
}

#[allow(dead_code)]
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
    fn test_clean_title_filename() {
        assert_eq!(clean_title_filename("Fields of gold"), "Fields of gold");
        assert_eq!(clean_title_filename("Mount Fuji: Japan / Sunset"), "Mount Fuji- Japan - Sunset");
        assert_eq!(clean_title_filename("  The  \"Great\"  Wave  "), "The Great Wave");
        assert_eq!(clean_title_filename("???"), "Bing Wallpaper");
        assert_eq!(clean_title_filename(""), "Bing Wallpaper");
    }

    #[test]
    fn test_directories_non_empty() {
        let wall_dir = wallpapers_online_dir();
        let thumb_dir = thumbs_online_cache_dir();
        assert!(wall_dir.to_string_lossy().contains("Wallpapers/Aura") || wall_dir.to_string_lossy().contains("aura"));
        assert!(thumb_dir.to_string_lossy().contains("online_thumbs"));
    }
}

