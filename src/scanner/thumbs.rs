use std::path::{Path, PathBuf};
use tokio::process::Command;

pub fn get_cache_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".cache").join("aura").join("thumbs")
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

pub async fn generate_thumbnail(video_path: &Path) -> Option<PathBuf> {
    let thumb_path = thumb_path_for_video(video_path);
    if thumb_path.exists() {
        return Some(thumb_path);
    }

    if let Some(parent) = thumb_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss")
        .arg("00:00:01")
        .arg("-i")
        .arg(video_path)
        .arg("-vframes")
        .arg("1")
        .arg("-vf")
        .arg("scale=480:-1")
        .arg(&thumb_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;

    match status {
        Ok(s) if s.success() && thumb_path.exists() => Some(thumb_path),
        _ => None,
    }
}
