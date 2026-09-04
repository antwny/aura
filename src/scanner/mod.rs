pub mod thumbs;

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "webm", "mkv", "avi", "mov"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoItem {
    pub path: PathBuf,
    pub name: String,
    pub size_formatted: String,
    pub thumb_path: Option<PathBuf>,
}

pub fn scan_directories(dirs: &[String], extra_files: &[String]) -> Vec<VideoItem> {
    let mut items = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    // 1. Scan configured directories
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
                        let canonical = path.to_path_buf();
                        if seen_paths.insert(canonical.clone()) {
                            let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Video".into());
                            let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                            let size_formatted = format_file_size(size_bytes);

                            let potential_thumb = thumbs::thumb_path_for_video(path);
                            let thumb_path = if potential_thumb.exists() {
                                Some(potential_thumb)
                            } else {
                                None
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
                let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Video".into());
                let size_bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
                let size_formatted = format_file_size(size_bytes);

                let potential_thumb = thumbs::thumb_path_for_video(path);
                let thumb_path = if potential_thumb.exists() {
                    Some(potential_thumb)
                } else {
                    None
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

fn format_file_size(bytes: u64) -> String {
    if bytes > 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes > 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    }
}
