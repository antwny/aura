pub mod thumbs;

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "webm", "mkv", "avi", "mov"];
pub const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

pub fn is_supported_wallpaper_ext(ext: &str) -> bool {
    let lower = ext.to_lowercase();
    VIDEO_EXTENSIONS.contains(&lower.as_str()) || IMAGE_EXTENSIONS.contains(&lower.as_str())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoItem {
    pub path: PathBuf,
    pub name: String,
    pub size_formatted: String,
    pub thumb_path: Option<PathBuf>,
}

impl VideoItem {
    pub fn is_video(&self) -> bool {
        self.path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false)
    }

    pub fn is_image(&self) -> bool {
        self.path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false)
    }

    pub fn is_downloaded(&self) -> bool {
        let online_dir = crate::online::wallpapers_online_dir();
        self.path.starts_with(&online_dir)
    }
}

pub fn scan_directories(dirs: &[String], extra_files: &[String]) -> Vec<VideoItem> {
    let mut items = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    // Ensure the online wallpapers dir is always scanned
    let online_dir = crate::online::wallpapers_online_dir().to_string_lossy().to_string();
    let mut all_dirs = dirs.to_vec();
    if !all_dirs.contains(&online_dir) {
        all_dirs.insert(0, online_dir);
    }

    // 1. Scan configured directories
    for dir_str in &all_dirs {
        let dir = Path::new(dir_str);
        if !dir.exists() {
            continue;
        }

        for entry in WalkDir::new(dir).max_depth(2).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if is_supported_wallpaper_ext(ext) {
                        let canonical = path.to_path_buf();
                        if seen_paths.insert(canonical.clone()) {
                            let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Wallpaper".into());
                            let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                            let size_formatted = format_file_size(size_bytes);

                            let thumb_path = if IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                                Some(path.to_path_buf())
                            } else {
                                let potential_thumb = thumbs::thumb_path_for_video(path);
                                if potential_thumb.exists() {
                                    Some(potential_thumb)
                                } else {
                                    None
                                }
                            };

                            items.push(VideoItem {
                                path: canonical,
                                name,
                                size_formatted,
                                thumb_path,
                            });
                        }
                    }
                }
            }
        }
    }

    // 2. Scan individually added custom files (via XDG Picker or Drag & Drop)
    for file_str in extra_files {
        let path = Path::new(file_str);
        if path.is_file() {
            let canonical = path.to_path_buf();
            if seen_paths.insert(canonical.clone()) {
                let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Wallpaper".into());
                let size_bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
                let size_formatted = format_file_size(size_bytes);

                let is_image = path.extension().and_then(|e| e.to_str()).map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str())).unwrap_or(false);
                let thumb_path = if is_image {
                    Some(path.to_path_buf())
                } else {
                    let potential_thumb = thumbs::thumb_path_for_video(path);
                    if potential_thumb.exists() {
                        Some(potential_thumb)
                    } else {
                        None
                    }
                };

                items.push(VideoItem {
                    path: canonical,
                    name,
                    size_formatted,
                    thumb_path,
                });
            }
        }
    }

    // Sort alphabetically by name
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    items
}

pub fn format_file_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(1024), "1 KB");
        assert_eq!(format_file_size(1536), "2 KB");
        assert_eq!(format_file_size(10 * 1024 * 1024), "10.0 MB");
        assert_eq!(format_file_size(2 * 1024 * 1024 * 1024), "2.0 GB");
    }

    #[test]
    fn test_video_extensions() {
        assert!(VIDEO_EXTENSIONS.contains(&"mp4"));
        assert!(VIDEO_EXTENSIONS.contains(&"webm"));
        assert!(VIDEO_EXTENSIONS.contains(&"mkv"));
        assert!(VIDEO_EXTENSIONS.contains(&"avi"));
        assert!(VIDEO_EXTENSIONS.contains(&"mov"));
        assert!(!VIDEO_EXTENSIONS.contains(&"png"));
        assert!(!VIDEO_EXTENSIONS.contains(&"exe"));
    }

    #[test]
    fn test_image_and_supported_extensions() {
        assert!(IMAGE_EXTENSIONS.contains(&"jpg"));
        assert!(IMAGE_EXTENSIONS.contains(&"jpeg"));
        assert!(IMAGE_EXTENSIONS.contains(&"png"));
        assert!(IMAGE_EXTENSIONS.contains(&"webp"));
        assert!(is_supported_wallpaper_ext("mp4"));
        assert!(is_supported_wallpaper_ext("JPG"));
        assert!(is_supported_wallpaper_ext("PNG"));
        assert!(!is_supported_wallpaper_ext("exe"));
        assert!(!is_supported_wallpaper_ext("txt"));
    }

    #[test]
    fn test_video_item_methods() {
        let vid = VideoItem {
            path: PathBuf::from("/home/antwny/Videos/cool.mp4"),
            name: "cool".into(),
            size_formatted: "10 MB".into(),
            thumb_path: None,
        };
        assert!(vid.is_video());
        assert!(!vid.is_image());
        assert!(!vid.is_downloaded());

        let img = VideoItem {
            path: crate::online::wallpapers_online_dir().join("bing_123.jpg"),
            name: "bing_123".into(),
            size_formatted: "2 MB".into(),
            thumb_path: None,
        };
        assert!(!img.is_video());
        assert!(img.is_image());
        assert!(img.is_downloaded());
    }
}

