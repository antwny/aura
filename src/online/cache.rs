use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn wallpapers_online_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let dir = PathBuf::from(home).join(".local/share/aura/wallpapers/online");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn thumbs_online_cache_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let dir = PathBuf::from(home).join(".cache/aura/online_thumbs");
    let _ = std::fs::create_dir_all(&dir);
    dir
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
        .header("User-Agent", "Aura-LiveWallpaper-Client/0.1.0")
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
        assert!(wall_dir.to_string_lossy().contains("wallpapers/online"));
        assert!(thumb_dir.to_string_lossy().contains("online_thumbs"));
    }
}

