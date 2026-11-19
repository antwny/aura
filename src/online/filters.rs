use super::OnlineSource;

/// Categories that can be applied across every catalogue. Each site exposes
/// a different slug (or no category at all); the mapping below is best-effort
/// and sources that cannot honour a category are skipped by the unified view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryFilter {
    Anime,
    Games,
    Movies,
    Nature,
    Abstract,
    SciFi,
    Fantasy,
    Space,
    Cars,
    Superhero,
    Technology,
    General,
}

impl CategoryFilter {
    pub const ALL: [CategoryFilter; 12] = [
        CategoryFilter::Anime,
        CategoryFilter::Games,
        CategoryFilter::Movies,
        CategoryFilter::Nature,
        CategoryFilter::Abstract,
        CategoryFilter::SciFi,
        CategoryFilter::Fantasy,
        CategoryFilter::Space,
        CategoryFilter::Cars,
        CategoryFilter::Superhero,
        CategoryFilter::Technology,
        CategoryFilter::General,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionFilter {
    Hd1080,
    Qhd2K,
    Uhd4K,
    Uhd8K,
    Ultrawide,
}

impl ResolutionFilter {
    pub const ALL: [ResolutionFilter; 5] = [
        ResolutionFilter::Hd1080,
        ResolutionFilter::Qhd2K,
        ResolutionFilter::Uhd4K,
        ResolutionFilter::Uhd8K,
        ResolutionFilter::Ultrawide,
    ];

    pub fn key(self) -> &'static str {
        match self {
            ResolutionFilter::Hd1080 => "1080p",
            ResolutionFilter::Qhd2K => "2k",
            ResolutionFilter::Uhd4K => "4k",
            ResolutionFilter::Uhd8K => "8k",
            ResolutionFilter::Ultrawide => "ultrawide",
        }
    }
}

/// Native category slug for a source, or `None` when the catalogue cannot
/// filter by that category.
pub fn category_slug(source: OnlineSource, category: CategoryFilter) -> Option<&'static str> {
    match source {
        OnlineSource::MotionBGS => match category {
            CategoryFilter::Anime => Some("anime"),
            CategoryFilter::Games => Some("games"),
            CategoryFilter::Nature => Some("nature"),
            CategoryFilter::Space => Some("space"),
            CategoryFilter::Cars => Some("car"),
            CategoryFilter::Superhero => Some("superhero"),
            CategoryFilter::Fantasy => Some("fantasy"),
            CategoryFilter::Technology => Some("technology"),
            _ => None,
        },
        OnlineSource::LiveWallpapers4Free => match category {
            CategoryFilter::Anime => Some("anime-live-wallpapers"),
            CategoryFilter::Games => Some("games-live-wallpapers"),
            CategoryFilter::Fantasy => Some("fantasy-live-wallpapers"),
            CategoryFilter::Movies => Some("movies-tv-live-wallpapers"),
            CategoryFilter::Abstract => Some("abstract-live-wallpapers"),
            CategoryFilter::SciFi => Some("sci-fi-live-wallpapers"),
            CategoryFilter::Nature => Some("nature-live-wallpapers"),
            _ => None,
        },
        OnlineSource::Wallhaven => match category {
            CategoryFilter::Anime => Some("010"),
            CategoryFilter::General => Some("110"),
            _ => None,
        },
        OnlineSource::DesktopHut => match category {
            CategoryFilter::Anime => Some("anime-live-wallpapers"),
            CategoryFilter::Games => Some("games-live-wallpapers"),
            CategoryFilter::Nature => Some("nature-live-wallpapers"),
            CategoryFilter::Fantasy | CategoryFilter::SciFi => {
                Some("sci-fi-fantasy-live-wallpapers")
            }
            CategoryFilter::Movies => Some("movies-and-tv-live-wallpapers"),
            CategoryFilter::Abstract => Some("abstract-live-wallpapers"),
            CategoryFilter::Cars => Some("cars-motorcycles-live-wallpapers"),
            CategoryFilter::Technology => Some("tech-live-wallpapers"),
            CategoryFilter::Superhero => Some("comics-live-wallpapers"),
            CategoryFilter::Space | CategoryFilter::General => None,
        },
        OnlineSource::MoeWalls => match category {
            CategoryFilter::Anime => Some("anime"),
            CategoryFilter::Games => Some("games"),
            CategoryFilter::Nature => Some("landscape"),
            CategoryFilter::Fantasy => Some("fantasy"),
            CategoryFilter::SciFi => Some("sci-fi"),
            CategoryFilter::Movies => Some("movies"),
            CategoryFilter::Abstract => Some("abstract"),
            CategoryFilter::Cars => Some("vehicle"),
            _ => None,
        },
        OnlineSource::WallpaperWaifu => match category {
            CategoryFilter::Anime => Some("anime"),
            CategoryFilter::Games => Some("games"),
            CategoryFilter::Nature => Some("landscape"),
            CategoryFilter::Fantasy => Some("fantasy"),
            CategoryFilter::SciFi => Some("sci-fi"),
            CategoryFilter::Movies => Some("movies"),
            CategoryFilter::Abstract => Some("abstract"),
            CategoryFilter::Cars => Some("vehicles"),
            _ => None,
        },
        _ => None,
    }
}

/// Server-side resolution slug for a source, or `None` when the catalogue
/// does not expose a resolution filter (the unified view post-filters instead).
pub fn resolution_slug(
    source: OnlineSource,
    resolution: ResolutionFilter,
) -> Option<&'static str> {
    match source {
        OnlineSource::MotionBGS => match resolution {
            ResolutionFilter::Uhd4K => Some("4k"),
            ResolutionFilter::Hd1080 => Some("hd"),
            _ => None,
        },
        OnlineSource::LiveWallpapers4Free => match resolution {
            ResolutionFilter::Hd1080 => Some("hd"),
            ResolutionFilter::Qhd2K => Some("2k"),
            ResolutionFilter::Uhd4K => Some("4k"),
            ResolutionFilter::Uhd8K => Some("8k"),
            ResolutionFilter::Ultrawide => Some("qhd-ultrawide"),
        },
        OnlineSource::Wallhaven => match resolution {
            ResolutionFilter::Hd1080 => Some("all"),
            ResolutionFilter::Qhd2K => Some("2k"),
            ResolutionFilter::Uhd4K => Some("4k"),
            ResolutionFilter::Ultrawide => Some("ultrawide"),
            ResolutionFilter::Uhd8K => Some("all"),
        },
        _ => None,
    }
}

/// Client-side fallback used for every catalogue once results arrive. Unknown
/// resolutions (for example "Live") are kept so a filter never empties a
/// source that simply does not publish dimensions.
pub fn resolution_matches(raw: &str, filter: ResolutionFilter) -> bool {
    let value = raw.trim().to_ascii_lowercase();
    if value.is_empty() || value == "live" || value == "unknown" {
        return true;
    }
    let has = |needle: &str| value.contains(needle);
    match filter {
        ResolutionFilter::Hd1080 => {
            has("1080") || has("1920x1080") || (has("hd") && !has("uhd") && !has("qhd"))
        }
        ResolutionFilter::Qhd2K => has("2k") || has("1440") || has("2560x1440"),
        ResolutionFilter::Uhd4K => {
            has("4k") || has("2160") || has("3840x2160") || has("5k") || has("8k")
        }
        ResolutionFilter::Uhd8K => has("8k") || has("4320") || has("7680x4320"),
        ResolutionFilter::Ultrawide => {
            has("ultrawide") || has("21:9") || has("3440x1440") || has("5120x1440")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_categories_per_source() {
        assert_eq!(
            category_slug(OnlineSource::MotionBGS, CategoryFilter::Cars),
            Some("car")
        );
        assert_eq!(
            category_slug(OnlineSource::LiveWallpapers4Free, CategoryFilter::SciFi),
            Some("sci-fi-live-wallpapers")
        );
        assert_eq!(
            category_slug(OnlineSource::Wallhaven, CategoryFilter::Anime),
            Some("010")
        );
        assert_eq!(
            category_slug(OnlineSource::MoeWalls, CategoryFilter::Movies),
            Some("movies")
        );
        assert_eq!(
            category_slug(OnlineSource::DesktopHut, CategoryFilter::Anime),
            Some("anime-live-wallpapers")
        );
        assert_eq!(
            category_slug(OnlineSource::DesktopHut, CategoryFilter::SciFi),
            Some("sci-fi-fantasy-live-wallpapers")
        );
        assert_eq!(
            category_slug(OnlineSource::MoeWalls, CategoryFilter::Nature),
            Some("landscape")
        );
        assert_eq!(
            category_slug(OnlineSource::WallpaperWaifu, CategoryFilter::Cars),
            Some("vehicles")
        );
        assert_eq!(category_slug(OnlineSource::Lively, CategoryFilter::Anime), None);
        assert_eq!(
            category_slug(OnlineSource::Wallhaven, CategoryFilter::Nature),
            None
        );
    }

    #[test]
    fn matches_common_resolution_strings() {
        for raw in ["4K UHD", "3840x2160", "2160p", "5120x2160"] {
            assert!(resolution_matches(raw, ResolutionFilter::Uhd4K), "{raw}");
        }
        for raw in ["2K", "2560x1440", "1440p"] {
            assert!(resolution_matches(raw, ResolutionFilter::Qhd2K), "{raw}");
        }
        for raw in ["1080p", "1920x1080", "HD"] {
            assert!(resolution_matches(raw, ResolutionFilter::Hd1080), "{raw}");
        }
        assert!(resolution_matches(
            "3440x1440",
            ResolutionFilter::Ultrawide
        ));
        assert!(resolution_matches("8K", ResolutionFilter::Uhd8K));
        assert!(resolution_matches("Live", ResolutionFilter::Uhd4K));
        assert!(!resolution_matches("1920x1080", ResolutionFilter::Uhd4K));
    }
}
