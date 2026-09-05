use std::path::{Path, PathBuf};
use tokio::process::Command;

pub fn get_cache_dir() -> PathBuf {
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
    base.join("aura/thumbs")
}

pub fn thumb_path_for_video(video_path: &Path) -> PathBuf {
    let cache_dir = get_cache_dir();
    let file_stem = video_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "video".into());
    // Sanitize name and add hash based on full path to avoid collision
    let clean_stem: String = file_stem.chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').take(60).collect();
    let hash = format!("{:016x}", md5_simple(video_path.to_string_lossy().as_bytes()));
    cache_dir.join(format!("{}_{}.jpg", clean_stem, &hash[..8]))
}

fn md5_simple(bytes: &[u8]) -> u64 {
    // Fast lightweight FNV-1a hash for deterministic thumb names
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_simple_deterministic() {
        let h1 = md5_simple(b"/home/user/Videos/wallpaper.mp4");
        let h2 = md5_simple(b"/home/user/Videos/wallpaper.mp4");
        let h3 = md5_simple(b"/home/user/Videos/other.mp4");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_thumb_path_for_video() {
        let p = Path::new("/path/to/my video file!.mp4");
        let thumb = thumb_path_for_video(p);
        let file_name = thumb.file_name().unwrap().to_str().unwrap();
        assert!(file_name.starts_with("myvideofile_"));
        assert!(file_name.ends_with(".jpg"));
        assert!(thumb.starts_with(get_cache_dir()));
    }

    #[test]
    fn test_hash_leading_zeros_safety() {
        let hash = format!("{:016x}", 0u64);
        assert_eq!(hash.len(), 16);
        assert_eq!(&hash[..8], "00000000");
    }
}

static THUMB_SEMAPHORE: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();

pub async fn generate_thumbnail(media_path: &Path) -> Option<PathBuf> {
    let thumb_path = thumb_path_for_video(media_path);
    if thumb_path.exists() {
        return Some(thumb_path);
    }

    let sem = THUMB_SEMAPHORE.get_or_init(|| tokio::sync::Semaphore::new(2));
    let _permit = sem.acquire().await.ok()?;

    if thumb_path.exists() {
        return Some(thumb_path);
    }

    if let Some(parent) = thumb_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    let is_image = media_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| crate::scanner::IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false);

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y");
    if !is_image {
        cmd.arg("-ss").arg("00:00:01");
    }
    cmd.arg("-i").arg(media_path);
    if !is_image {
        cmd.arg("-an").arg("-sn");
    }
    cmd.arg("-vframes")
        .arg("1")
        .arg("-vf")
        .arg("scale=480:-1")
        .arg(&thumb_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let status = cmd.status().await;

    match status {
        Ok(s) if s.success() && thumb_path.exists() => Some(thumb_path),
        _ => None,
    }
}
