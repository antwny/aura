use super::models::{OnlineSource, OnlineWallpaperItem};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

const API_BASE: &str = "https://livewallpapers4free.com/wp-json/wp/v2";
// MotionBGS serves 36 wallpaper cards on its paginated catalog pages.
const PAGE_SIZE: usize = 36;

/// Categories exposed by the site's public WordPress API, in Explore menu order.
pub const CATEGORIES: [(&str, &str); 9] = [
    ("all", "All"),
    ("anime-live-wallpapers", "Anime"),
    ("games-live-wallpapers", "Games"),
    ("fantasy-live-wallpapers", "Fantasy"),
    ("movies-tv-live-wallpapers", "Movies / TV"),
    ("abstract-live-wallpapers", "Abstract"),
    ("3d-live-wallpapers", "3D Animation"),
    ("sci-fi-live-wallpapers", "Sci-Fi"),
    ("nature-live-wallpapers", "Nature"),
];

/// Resolution choices shown by Explore. The site stores these as WordPress categories.
pub const RESOLUTIONS: [(&str, &str); 8] = [
    ("hd", "1080p HD"),
    ("2k", "2K (2560×1440)"),
    ("4k", "4K (3840×2160)"),
    ("8k", "8K (7680×4320)"),
    ("5k", "5K (5984×2560)"),
    ("5k2k", "5K2K (5120×2160)"),
    ("qhd-ultrawide", "QHD UltraWide (3440×1440)"),
    ("dqhd-ultrawide", "DQHD UltraWide (5120×1440)"),
];

#[derive(Debug, Clone)]
pub struct LiveWallpapers4FreePage {
    pub items: Vec<OnlineWallpaperItem>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quality {
    Hd,
    TwoK,
    FourK,
    EightK,
    FiveK,
    FiveK2K,
    QhdUltrawide,
    DqhdUltrawide,
}

impl Quality {
    fn from_key(key: &str) -> Option<Self> {
        match key {
            "hd" => Some(Self::Hd),
            "2k" => Some(Self::TwoK),
            "4k" => Some(Self::FourK),
            "8k" => Some(Self::EightK),
            "5k" => Some(Self::FiveK),
            "5k2k" => Some(Self::FiveK2K),
            "qhd-ultrawide" => Some(Self::QhdUltrawide),
            "dqhd-ultrawide" => Some(Self::DqhdUltrawide),
            _ => None,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Hd => "hd",
            Self::TwoK => "2k",
            Self::FourK => "4k",
            Self::EightK => "8k",
            Self::FiveK => "5k",
            Self::FiveK2K => "5k2k",
            Self::QhdUltrawide => "qhd-ultrawide",
            Self::DqhdUltrawide => "dqhd-ultrawide",
        }
    }

    fn label(self) -> &'static str {
        RESOLUTIONS
            .iter()
            .find_map(|(key, label)| (*key == self.key()).then_some(*label))
            .unwrap_or("Live Wallpaper")
    }

    fn rank(self) -> u8 {
        match self {
            Self::Hd => 1,
            Self::TwoK => 2,
            Self::QhdUltrawide => 3,
            Self::FourK => 4,
            Self::FiveK => 5,
            Self::FiveK2K => 6,
            Self::DqhdUltrawide => 7,
            Self::EightK => 8,
        }
    }

}

#[derive(Debug)]
struct DownloadVariant {
    quality: Quality,
    url: String,
}

static CATEGORY_IDS: tokio::sync::OnceCell<HashMap<String, u64>> =
    tokio::sync::OnceCell::const_new();

async fn category_ids(client: &reqwest::Client) -> Result<&'static HashMap<String, u64>, String> {
    CATEGORY_IDS
        .get_or_try_init(|| async {
            let response = client
                .get(format!("{API_BASE}/categories"))
                .query(&[("per_page", "100"), ("_fields", "id,slug")])
                .timeout(Duration::from_secs(15))
                .header("User-Agent", concat!("Aura-LiveWallpaper-Client/", env!("CARGO_PKG_VERSION")))
                .send()
                .await
                .map_err(|e| format!("Could not load LiveWallpapers4Free categories: {e}"))?;

            if !response.status().is_success() {
                return Err(format!(
                    "LiveWallpapers4Free categories returned HTTP {}",
                    response.status()
                ));
            }

            let values = response
                .json::<Vec<Value>>()
                .await
                .map_err(|e| format!("Could not parse LiveWallpapers4Free categories: {e}"))?;
            let mut ids = HashMap::new();
            for value in values {
                if let (Some(slug), Some(id)) = (
                    value.get("slug").and_then(Value::as_str),
                    value.get("id").and_then(Value::as_u64),
                ) {
                    ids.insert(slug.to_string(), id);
                }
            }
            Ok(ids)
        })
        .await
}

pub async fn fetch_livewallpapers4free_page(
    client: &reqwest::Client,
    page: u32,
    query: Option<&str>,
    category: &str,
    resolution: &str,
) -> Result<LiveWallpapers4FreePage, String> {
    let selected_category = if !category.is_empty() && category != "all" {
        let ids = category_ids(client).await?;
        let id = ids
            .get(category)
            .ok_or_else(|| format!("LiveWallpapers4Free category is unavailable: {category}"))?;
        Some(*id)
    } else {
        None
    };

    let quality = Quality::from_key(resolution).unwrap_or(Quality::FourK);

    let mut url = reqwest::Url::parse(&format!("{API_BASE}/posts"))
        .map_err(|e| format!("Invalid LiveWallpapers4Free API URL: {e}"))?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("page", &page.max(1).to_string())
            .append_pair("per_page", &PAGE_SIZE.to_string())
            .append_pair("_embed", "wp:featuredmedia")
            .append_pair(
                "_fields",
                "id,date,title,content,_links,_embedded",
            );
        if let Some(query) = query.map(str::trim).filter(|q| !q.is_empty()) {
            pairs.append_pair("search", query);
        }
        if let Some(id) = selected_category {
            pairs.append_pair("categories[]", &id.to_string());
        }
    }

    let response = client
        .get(url)
        .timeout(Duration::from_secs(20))
        .header("User-Agent", concat!("Aura-LiveWallpaper-Client/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Network request to LiveWallpapers4Free failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "LiveWallpapers4Free returned HTTP {}",
            response.status()
        ));
    }

    let total_pages = response
        .headers()
        .get("X-WP-TotalPages")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(page);
    let values = response
        .json::<Vec<Value>>()
        .await
        .map_err(|e| format!("Could not parse LiveWallpapers4Free response: {e}"))?;

    let mut items = Vec::new();
    for value in values {
        if let Some(item) = parse_post(&value, quality) {
            if !items.iter().any(|existing: &OnlineWallpaperItem| existing.id == item.id) {
                items.push(item);
            }
        }
    }

    if items.is_empty() && page == 1 {
        return Err("No live wallpapers found on LiveWallpapers4Free for this query.".into());
    }

    Ok(LiveWallpapers4FreePage {
        items,
        has_more: page < total_pages,
    })
}

fn parse_post(value: &Value, requested: Quality) -> Option<OnlineWallpaperItem> {
    let id = format!("lw4free_{}", value.get("id")?.as_u64()?);
    let title = decode_html_entities(value.get("title")?.get("rendered")?.as_str()?).trim().to_string();
    let content = value.get("content")?.get("rendered")?.as_str()?;
    let media = value
        .get("_embedded")?
        .get("wp:featuredmedia")?
        .as_array()?
        .first()?;
    let thumb_url = media
        .get("media_details")
        .and_then(|details| details.get("sizes"))
        .and_then(|sizes| sizes.get("medium"))
        .and_then(|medium| medium.get("source_url"))
        .and_then(Value::as_str)
        .or_else(|| media.get("source_url").and_then(Value::as_str))?
        .to_string();

    let variants = parse_download_variants(content);
    let selected = select_variant(&variants, requested)?;
    let actual_quality = selected.quality;
    let date = value
        .get("date")
        .and_then(Value::as_str)
        .and_then(|date| date.split('T').next())
        .map(str::to_string);

    Some(OnlineWallpaperItem {
        id,
        title,
        author_or_copyright: "LiveWallpapers4Free".into(),
        thumb_url,
        full_url: selected.url.clone(),
        resolution: actual_quality.label().to_string(),
        source: OnlineSource::LiveWallpapers4Free,
        date,
    })
}

fn parse_download_variants(html: &str) -> Vec<DownloadVariant> {
    let lower = html.to_ascii_lowercase();
    let mut variants = Vec::new();
    let mut cursor = 0;

    while let Some(relative) = lower[cursor..].find("<a") {
        let start = cursor + relative;
        let after_name = start + 2;
        if lower
            .as_bytes()
            .get(after_name)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>')
        {
            cursor = after_name;
            continue;
        }

        let Some(tag_end_relative) = lower[after_name..].find('>') else {
            break;
        };
        let tag_end = after_name + tag_end_relative;
        let tag = &html[start..=tag_end];
        let class = attribute(tag, "class").unwrap_or_default();
        let href = attribute(tag, "href").unwrap_or_default();
        let Some(close_relative) = lower[tag_end + 1..].find("</a>") else {
            cursor = tag_end + 1;
            continue;
        };
        let close_start = tag_end + 1 + close_relative;
        let text = plain_text(&html[tag_end + 1..close_start]);
        cursor = close_start + 4;

        if !class.contains("dlm-download-link") {
            continue;
        }

        let href = decode_html_entities(&href);
        let Ok(url) = reqwest::Url::parse(&href) else {
            continue;
        };
        if url.host_str() != Some("livewallpapers4free.com")
            || !url.path().starts_with("/download/")
        {
            continue;
        }

        let Some(quality) = detect_quality(&format!("{text} {href}")) else {
            continue;
        };
        if !variants.iter().any(|variant: &DownloadVariant| variant.quality == quality) {
            variants.push(DownloadVariant {
                quality,
                url: url.to_string(),
            });
        }
    }

    variants
}

fn select_variant(variants: &[DownloadVariant], requested: Quality) -> Option<&DownloadVariant> {
    variants
        .iter()
        .find(|variant| variant.quality == requested)
        .or_else(|| {
            variants
                .iter()
                .filter(|variant| variant.quality.rank() <= requested.rank())
                .max_by_key(|variant| variant.quality.rank())
        })
        .or_else(|| variants.iter().min_by_key(|variant| variant.quality.rank()))
}

fn detect_quality(value: &str) -> Option<Quality> {
    let value = value
        .to_ascii_lowercase()
        .replace('×', "x")
        .replace(' ', "");

    let suffix_markers = [
        ("-5k2k", Quality::FiveK2K),
        ("-dqhd", Quality::DqhdUltrawide),
        ("-qhd", Quality::QhdUltrawide),
        ("-8k", Quality::EightK),
        ("-5k", Quality::FiveK),
        ("-4k", Quality::FourK),
        ("-2k", Quality::TwoK),
        ("-hd", Quality::Hd),
    ];
    let suffix = suffix_markers
        .iter()
        .filter_map(|(marker, quality)| value.rfind(marker).map(|index| (index, *quality)))
        .max_by_key(|(index, _)| *index);
    if let Some((_, quality)) = suffix {
        return Some(quality);
    }

    if value.contains("5120x1440") || value.contains("dqhd") {
        Some(Quality::DqhdUltrawide)
    } else if value.contains("5120x2160") || value.contains("5k2k") {
        Some(Quality::FiveK2K)
    } else if value.contains("3440x1440") || value.contains("qhd-ultrawide") {
        Some(Quality::QhdUltrawide)
    } else if value.contains("7680x4320") || value.contains("8k") {
        Some(Quality::EightK)
    } else if value.contains("5984x2560") || value.contains("5k") {
        Some(Quality::FiveK)
    } else if value.contains("3840x2160") || value.contains("4k") {
        Some(Quality::FourK)
    } else if value.contains("2560x1440") || value.contains("2k") {
        Some(Quality::TwoK)
    } else if value.contains("1920x1080") || value.contains("1080p") || value.contains("-hd") || value.contains("_hd") {
        Some(Quality::Hd)
    } else {
        None
    }
}

fn attribute(tag: &str, requested: &str) -> Option<String> {
    let bytes = tag.as_bytes();
    let mut index = 2;
    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let key_start = index;
        while index < bytes.len()
            && !bytes[index].is_ascii_whitespace()
            && bytes[index] != b'='
            && bytes[index] != b'>'
        {
            index += 1;
        }
        if key_start == index {
            index += 1;
            continue;
        }
        let key = &tag[key_start..index];
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'=' {
            continue;
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            return None;
        }
        let quote = bytes[index];
        if quote == b'\'' || quote == b'"' {
            index += 1;
            let value_start = index;
            while index < bytes.len() && bytes[index] != quote {
                index += 1;
            }
            if key.eq_ignore_ascii_case(requested) {
                return Some(tag[value_start..index].to_string());
            }
            index = (index + 1).min(bytes.len());
        } else {
            let value_start = index;
            while index < bytes.len() && !bytes[index].is_ascii_whitespace() && bytes[index] != b'>' {
                index += 1;
            }
            if key.eq_ignore_ascii_case(requested) {
                return Some(tag[value_start..index].to_string());
            }
        }
    }
    None
}

fn plain_text(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }
    decode_html_entities(&text).split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_html_entities(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(index) = rest.find('&') {
        decoded.push_str(&rest[..index]);
        rest = &rest[index..];
        let Some(end) = rest.find(';') else {
            decoded.push_str(rest);
            return decoded;
        };
        let entity = &rest[1..end];
        let replacement = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                u32::from_str_radix(&entity[2..], 16).ok().and_then(char::from_u32)
            }
            _ if entity.starts_with('#') => entity[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        if let Some(ch) = replacement {
            decoded.push(ch);
        } else {
            decoded.push_str(&rest[..=end]);
        }
        rest = &rest[end + 1..];
    }
    decoded.push_str(rest);
    decoded
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CONTENT: &str = r#"
        <a class="kgvid-download-link" href="https://livewallpapers4free.com/wp-content/uploads/preview.mp4">Preview</a>
        <a class="dlm-download-link" href="https://livewallpapers4free.com/download/53887/">ada-wong-rainfall-HD-live.mp4 (download)</a>
        <a class="dlm-download-link" href="https://livewallpapers4free.com/download/53884/">ada-wong-rainfall-2k-live.mp4 (download)</a>
        <a class="dlm-download-link" href="https://livewallpapers4free.com/download/53881/">ada-wong-rainfall-4k-live.mp4 (download)</a>
    "#;

    #[test]
    fn parses_download_variants_and_ignores_preview_video() {
        let variants = parse_download_variants(SAMPLE_CONTENT);

        assert_eq!(variants.len(), 3);
        assert_eq!(select_variant(&variants, Quality::FourK).unwrap().url, "https://livewallpapers4free.com/download/53881/");
        assert_eq!(select_variant(&variants, Quality::EightK).unwrap().quality, Quality::FourK);
        assert_eq!(detect_quality("A 4K title - 2k-live.mp4"), Some(Quality::TwoK));
        assert_eq!(detect_quality("5120×1440"), Some(Quality::DqhdUltrawide));
    }

    #[test]
    fn parses_api_post_into_a_downloadable_mp4_item() {
        let post = serde_json::json!({
            "id": 53879,
            "date": "2026-11-05T11:01:09",
            "title": { "rendered": "Ada Wong &#8211; Rainfall" },
            "content": { "rendered": SAMPLE_CONTENT },
            "_embedded": {
                "wp:featuredmedia": [{
                    "source_url": "https://livewallpapers4free.com/wp-content/uploads/ada-wong-thumb.jpg",
                    "media_details": { "sizes": { "medium": {
                        "source_url": "https://livewallpapers4free.com/wp-content/uploads/ada-wong-thumb-300x169.jpg"
                    } } }
                }]
            }
        });

        let item = parse_post(&post, Quality::FourK).unwrap();
        assert_eq!(item.id, "lw4free_53879");
        assert_eq!(item.title, "Ada Wong – Rainfall");
        assert_eq!(item.resolution, "4K (3840×2160)");
        assert_eq!(item.full_url, "https://livewallpapers4free.com/download/53881/");
        assert_eq!(item.thumb_url, "https://livewallpapers4free.com/wp-content/uploads/ada-wong-thumb-300x169.jpg");
        assert_eq!(item.extension(), "mp4");
        assert_eq!(item.date.as_deref(), Some("2026-11-05"));
    }
}
