use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::online::cache::{clean_filename, clean_title_filename, thumbs_online_cache_dir, wallpapers_online_dir};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnlineSource {
    Bing,
    Wallhaven,
    Minimalistic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnlineWallpaperItem {
    pub id: String,
    pub title: String,
    pub author_or_copyright: String,
    pub thumb_url: String,
    pub full_url: String,
    pub resolution: String,
    pub source: OnlineSource,
    pub date: Option<String>,
}

impl OnlineWallpaperItem {
    pub fn extension(&self) -> &str {
        if self.full_url.ends_with(".png") {
            "png"
        } else if self.full_url.ends_with(".webp") {
            "webp"
        } else {
            "jpg"
        }
    }

    pub fn thumb_extension(&self) -> &str {
        if self.thumb_url.ends_with(".png") {
            "png"
        } else if self.thumb_url.ends_with(".webp") {
            "webp"
        } else {
            "jpg"
        }
    }

    pub fn source_prefix(&self) -> &'static str {
        match self.source {
            OnlineSource::Bing => "bing",
            OnlineSource::Wallhaven => "wallhaven",
            OnlineSource::Minimalistic => "minimal",
        }
    }

    pub fn wallpaper_filename(&self) -> String {
        match self.source {
            OnlineSource::Bing => {
                let clean = clean_title_filename(&self.title);
                if clean == "Bing Wallpaper" {
                    let short_id = if self.id.len() >= 8 { &self.id[..8] } else { &self.id };
                    format!("Bing Wallpaper - {}.{}", short_id, self.extension())
                } else {
                    format!("{}.{}", clean, self.extension())
                }
            }
            OnlineSource::Wallhaven => {
                clean_filename(self.source_prefix(), &self.id, self.extension())
            }
            OnlineSource::Minimalistic => {
                let clean_title = clean_title_filename(&self.title);
                let clean_author = clean_title_filename(&self.author_or_copyright);
                if !clean_author.is_empty() && clean_author != "Minimalist" {
                    format!("{} - {}.{}", clean_author, clean_title, self.extension())
                } else {
                    format!("{}.{}", clean_title, self.extension())
                }
            }
        }
    }

    pub fn local_wallpaper_path(&self) -> PathBuf {
        let dir = wallpapers_online_dir();
        dir.join(self.wallpaper_filename())
    }

    pub fn local_thumb_path(&self) -> PathBuf {
        let dir = thumbs_online_cache_dir();
        dir.join(clean_filename(self.source_prefix(), &self.id, self.thumb_extension()))
    }

    pub fn is_downloaded(&self) -> Option<PathBuf> {
        let dir = wallpapers_online_dir();

        // 1. Check title-based target path
        let target = self.local_wallpaper_path();
        if target.exists() {
            if let Ok(metadata) = target.metadata() {
                if metadata.len() > 1024 {
                    return Some(target);
                }
            }
        }

        // 2. Check legacy paths (e.g. "bing_xyz123.jpg") for backwards compatibility
        let extensions = ["jpg", "jpeg", "png", "webp"];
        for ext in &extensions {
            let legacy = dir.join(clean_filename(self.source_prefix(), &self.id, ext));
            if legacy.exists() {
                if let Ok(metadata) = legacy.metadata() {
                    if metadata.len() > 1024 {
                        if !target.exists() {
                            if std::fs::rename(&legacy, &target).is_ok() {
                                let old_thumb = crate::scanner::thumbs::thumb_path_for_video(&legacy);
                                let new_thumb = crate::scanner::thumbs::thumb_path_for_video(&target);
                                if old_thumb.exists() && !new_thumb.exists() {
                                    let _ = std::fs::rename(&old_thumb, &new_thumb);
                                }
                                return Some(target);
                            }
                        }
                        return Some(legacy);
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_extensions_and_prefix() {
        let item = OnlineWallpaperItem {
            id: "bing_123".into(),
            title: "Test Image".into(),
            author_or_copyright: "Bing".into(),
            thumb_url: "https://bing.com/thumb.jpg".into(),
            full_url: "https://bing.com/full_uhd.png".into(),
            resolution: "3840x2160".into(),
            source: OnlineSource::Bing,
            date: Some("2026-09-04".into()),
        };

        assert_eq!(item.extension(), "png");
        assert_eq!(item.thumb_extension(), "jpg");
        assert_eq!(item.source_prefix(), "bing");
        assert_eq!(item.wallpaper_filename(), "Test Image.png");
        assert!(item.local_wallpaper_path().to_string_lossy().ends_with("Test Image.png"));
        assert!(item.local_thumb_path().to_string_lossy().ends_with("bing_bing_123.jpg"));

        let minimal_item = OnlineWallpaperItem {
            id: "minimal_alena-aenami-clouds_jpg".into(),
            title: "Clouds".into(),
            author_or_copyright: "Alena Aenami".into(),
            thumb_url: "https://raw.githubusercontent.com/test.jpg".into(),
            full_url: "https://raw.githubusercontent.com/test.jpg".into(),
            resolution: "2K QHD".into(),
            source: OnlineSource::Minimalistic,
            date: None,
        };

        assert_eq!(minimal_item.source_prefix(), "minimal");
        assert_eq!(minimal_item.wallpaper_filename(), "Alena Aenami - Clouds.jpg");
    }
}


