use std::time::Duration;
use super::models::{OnlineSource, OnlineWallpaperItem};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct WallhavenPage {
    pub items: Vec<OnlineWallpaperItem>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize)]
struct WallhavenMeta {
    current_page: Option<u32>,
    last_page: Option<u32>,
    #[allow(dead_code)]
    total: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct WallhavenResponse {
    data: Option<Vec<WallhavenItem>>,
    meta: Option<WallhavenMeta>,
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

pub async fn fetch_wallhaven_page(
    client: &reqwest::Client,
    page: u32,
    query: Option<&str>,
    category: &str,
    sorting: &str,
    resolution: &str,
) -> Result<WallhavenPage, String> {
    let mut params = vec![
        ("categories", category.to_string()),
        ("purity", "100".to_string()),
        ("page", page.to_string()),
    ];

    if let Some(q) = query {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            params.push(("q", trimmed.to_string()));
            params.push(("sorting", "relevance".to_string()));
        } else {
            params.push(("sorting", sorting.to_string()));
        }
    } else {
        params.push(("sorting", sorting.to_string()));
    }

    // NOTE: We intentionally do NOT send `atleast` or `topRange` to Wallhaven.
    // Their backend is unstable and returns 502 when these params are combined
    // with sorting/search. Aura already filters by resolution locally, so
    // server-side filtering is unnecessary. Only `ratios` is safe for ultrawide.
    if resolution == "ultrawide" {
        params.push(("ratios", "21x9,32x9".to_string()));
    }

    // Wallhaven's backend fails with HTTP/2 (Cloudflare returns instant 502).
    // Use a dedicated HTTP/1.1-only client to avoid this issue.
    static WALLHAVEN_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    let wh_client = WALLHAVEN_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .http1_only()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| client.clone())
    });

    let send_req = || {
        wh_client
            .get("https://wallhaven.cc/api/v1/search")
            .query(&params)
            .timeout(Duration::from_secs(8))
            .header(reqwest::header::ACCEPT, "application/json")
            .header(
                reqwest::header::USER_AGENT,
                concat!("Aura-LiveWallpaper-Client/", env!("CARGO_PKG_VERSION")),
            )
            .send()
    };

    let mut resp = send_req()
        .await
        .map_err(|e| format!("Network request to Wallhaven failed: {}", e))?;

    // Wallhaven's backend is very unstable behind Cloudflare and returns
    // intermittent 502/503/504 errors (~60-70% failure rate during outages).
    // Retry up to 3 times with exponential backoff.
    {
        let backoff_ms = [600, 1200, 2000];
        for delay in backoff_ms {
            if resp.status().is_success() {
                break;
            }
            if !resp.status().is_server_error() && resp.status().as_u16() != 429 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(delay)).await;
            match send_req().await {
                Ok(r) => resp = r,
                Err(_) => break,
            }
        }
    }

    if !resp.status().is_success() {
        return Err(format!("Wallhaven API returned error code {}", resp.status()));
    }

    let parsed = resp
        .json::<WallhavenResponse>()
        .await
        .map_err(|e| format!("Failed to parse Wallhaven data: {}", e))?;

    let has_more = parsed.meta.as_ref().map_or(false, |m| {
        let curr = m.current_page.unwrap_or(page);
        let last = m.last_page.unwrap_or(page);
        curr < last
    });

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

    Ok(WallhavenPage { items, has_more })
}

pub async fn fetch_wallhaven_wallpapers(
    client: &reqwest::Client,
    page: u32,
    query: Option<&str>,
    category: &str,
    sorting: &str,
    resolution: &str,
) -> Result<Vec<OnlineWallpaperItem>, String> {
    fetch_wallhaven_page(client, page, query, category, sorting, resolution)
        .await
        .map(|page| page.items)
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
            ],
            "meta": {
                "current_page": 1,
                "last_page": 3,
                "per_page": 24,
                "total": 72
            }
        }"#;

        let parsed: WallhavenResponse = serde_json::from_str(json_data).expect("Should deserialize Wallhaven JSON");
        assert!(parsed.data.is_some());
        let items = parsed.data.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "x9k123");
        assert_eq!(items[0].resolution.as_deref(), Some("3840x2160"));
        assert_eq!(parsed.meta.unwrap().last_page, Some(3));
    }

    #[test]
    fn test_parse_wallhaven_empty_data() {
        let json_data = r#"{
            "data": [],
            "meta": {
                "current_page": 1,
                "last_page": 1,
                "per_page": 24,
                "total": 0
            }
        }"#;

        let parsed: WallhavenResponse = serde_json::from_str(json_data).expect("Should deserialize empty Wallhaven JSON");
        assert!(parsed.data.is_some());
        assert!(parsed.data.unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_wallhaven_direct() {
        let client = reqwest::Client::new();
        let start = std::time::Instant::now();
        let result = fetch_wallhaven_page(&client, 1, None, "110", "toplist", "all").await;
        eprintln!("Wallhaven direct: is_ok={} in {:?}", result.is_ok(), start.elapsed());
        if let Err(e) = &result {
            eprintln!("Error: {e}");
        } else if let Ok(p) = &result {
            eprintln!("Items: {}, has_more: {}", p.items.len(), p.has_more);
        }
    }
}

