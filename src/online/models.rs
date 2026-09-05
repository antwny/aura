use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::online::cache::{clean_filename, is_downloaded, thumbs_online_cache_dir, wallpapers_online_dir};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnlineSource {
    Bing,
    Wallhaven,
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
        }
    }

    pub fn local_wallpaper_path(&self) -> PathBuf {
        let dir = wallpapers_online_dir();
        dir.join(clean_filename(self.source_prefix(), &self.id, self.extension()))
    }

    pub fn local_thumb_path(&self) -> PathBuf {
        let dir = thumbs_online_cache_dir();
        dir.join(clean_filename(self.source_prefix(), &self.id, self.thumb_extension()))
    }

    pub fn is_downloaded(&self) -> Option<PathBuf> {
        is_downloaded(self.source_prefix(), &self.id)
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
        assert!(item.local_wallpaper_path().to_string_lossy().ends_with("bing_bing_123.png"));
        assert!(item.local_thumb_path().to_string_lossy().ends_with("bing_bing_123.jpg"));
    }
}


