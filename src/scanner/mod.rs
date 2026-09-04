pub mod thumbs;

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "webm", "mkv", "avi", "mov"];

#[derive(Debug, Clone)]
pub struct VideoItem {
    pub path: PathBuf,
    pub name: String,
    pub size_formatted: String,
    pub thumb_path: Option<PathBuf>,
}

pub fn scan_directories(dirs: &[String]) -> Vec<VideoItem> {
    let mut items = Vec::new();

    for dir_str in dirs {
        let dir = Path::new(dir_str);
        if !dir.exists() {
            continue;
        }

        for entry in WalkDir::new(dir).max_depth(2).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Video".into());
                        let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        let size_formatted = if size_bytes > 1024 * 1024 * 1024 {
                            format!("{:.1} GB", size_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
                        } else if size_bytes > 1024 * 1024 {
                            format!("{:.1} MB", size_bytes as f64 / (1024.0 * 1024.0))
                        } else {
                            format!("{:.0} KB", size_bytes as f64 / 1024.0)
                        };

                        let potential_thumb = thumbs::thumb_path_for_video(path);
                        let thumb_path = if potential_thumb.exists() {
                            Some(potential_thumb)
                        } else {
                            None
                        };

                        items.push(VideoItem {
                            path: path.to_path_buf(),
                            name,
                            size_formatted,
                            thumb_path,
                        });
                    }
                }
            }
        }
    }

    // Sort alphabetically by name
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    items
}
