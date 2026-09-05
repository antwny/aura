use std::time::Duration;
use super::models::{OnlineSource, OnlineWallpaperItem};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct WallhavenResponse {
    data: Option<Vec<WallhavenItem>>,
}

#[derive(Debug, Deserialize)]
struct WallhavenItem {
    id: String,
    resolution: Option<String>,
    path: Option<String>,
    thumbs: Option<WallhavenThumbs>,
    category: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WallhavenThumbs {
    large: Option<String>,
    small: Option<String>,
}

pub async fn fetch_wallhaven_wallpapers(client: &reqwest::Client) -> Result<Vec<OnlineWallpaperItem>, String> {
    let url = "https://wallhaven.cc/api/v1/search?categories=110&purity=100&sorting=toplist&topRange=1M&resolutions=1920x1080,2560x1440,3840x2160";

    let resp = client
        .get(url)
        .timeout(Duration::from_secs(10))
        .header("User-Agent", "Aura-LiveWallpaper-Client/0.1.0")
        .send()
        .await
        .map_err(|e| format!("Network request to Wallhaven failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Wallhaven API returned error code {}", resp.status()));
    }

    let parsed = resp
        .json::<WallhavenResponse>()
        .await
        .map_err(|e| format!("Failed to parse Wallhaven data: {}", e))?;

    let mut items = Vec::new();
    if let Some(data) = parsed.data {
        for entry in data {
            let full_url = match entry.path {
                Some(p) if !p.is_empty() => p,
                _ => continue,
            };

            let thumb_url = match entry.thumbs {
                Some(t) => t.large.or(t.small).unwrap_or_else(|| full_url.clone()),
                None => full_url.clone(),
            };

            let category_label = entry.category.unwrap_or_else(|| "Artwork".into());
            let resolution = entry.resolution.unwrap_or_else(|| "High Resolution".into());

            items.push(OnlineWallpaperItem {
                id: entry.id.clone(),
                title: format!("Artwork #{}", entry.id),
                author_or_copyright: format!("Wallhaven ({})", category_label),
                thumb_url,
                full_url,
                resolution,
                source: OnlineSource::Wallhaven,
                date: None,
            });
        }
    }

    if items.is_empty() {
        return Err("No wallpapers found on Wallhaven at this time.".into());
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wallhaven_response() {
        let json_data = r#"{
            "data": [
                {
                    "id": "x9k123",
                    "resolution": "3840x2160",
                    "path": "https://w.wallhaven.cc/full/x9/wallhaven-x9k123.jpg",
                    "category": "anime",
                    "thumbs": {
                        "large": "https://th.wallhaven.cc/lg/x9/x9k123.jpg",
                        "small": "https://th.wallhaven.cc/small/x9/x9k123.jpg"
                    }
                }
            ]
        }"#;

        let parsed: WallhavenResponse = serde_json::from_str(json_data).expect("Should deserialize Wallhaven JSON");
        assert!(parsed.data.is_some());
        let items = parsed.data.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "x9k123");
        assert_eq!(items[0].resolution.as_deref(), Some("3840x2160"));
    }
}

