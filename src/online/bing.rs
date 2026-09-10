use std::time::Duration;
use super::models::{OnlineSource, OnlineWallpaperItem};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct BingApiResponse {
    images: Option<Vec<BingImage>>,
}

#[derive(Debug, Clone, Deserialize)]
struct BingImage {
    urlbase: Option<String>,
    copyright: Option<String>,
    title: Option<String>,
    startdate: Option<String>,
    hsh: Option<String>,
}

pub async fn fetch_bing_wallpapers(client: &reqwest::Client) -> Result<Vec<OnlineWallpaperItem>, String> {
    let url_page1 = "https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=8&mkt=en-US";
    let url_page2 = "https://www.bing.com/HPImageArchive.aspx?format=js&idx=8&n=8&mkt=en-US";

    let (res1, res2) = tokio::join!(
        client.get(url_page1).header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64)").timeout(Duration::from_secs(10)).send(),
        client.get(url_page2).header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64)").timeout(Duration::from_secs(10)).send()
    );

    let mut items = Vec::new();
    let cache_dir = crate::online::cache::thumbs_online_cache_dir();

    if let Ok(resp) = res1 {
        if resp.status().is_success() {
            if let Ok(text) = resp.text().await {
                let _ = tokio::fs::write(cache_dir.join("bing_daily.json"), &text).await;
                if let Ok(data) = serde_json::from_str::<BingApiResponse>(&text) {
                    if let Some(imgs) = data.images {
                        for img in imgs {
                            if let Some(item) = convert_bing_image(img) {
                                items.push(item);
                            }
                        }
                    }
                }
            }
        }
    }

    if let Ok(resp) = res2 {
        if resp.status().is_success() {
            if let Ok(data) = resp.json::<BingApiResponse>().await {
                if let Some(imgs) = data.images {
                    for img in imgs {
                        if let Some(item) = convert_bing_image(img) {
                            if !items.iter().any(|existing| existing.id == item.id) {
                                items.push(item);
                            }
                        }
                    }
                }
            }
        }
    }

    if items.is_empty() {
        return Err("Could not load wallpapers from Bing. Check your internet connection.".into());
    }

    Ok(items)
}

#[derive(Debug, Deserialize)]
struct BingArchiveRoot {
    data: Option<Vec<BingImage>>,
}

pub async fn fetch_bing_archive_page(
    client: &reqwest::Client,
    page: u32,
    per_page: usize,
) -> Result<Vec<OnlineWallpaperItem>, String> {
    let cache_dir = crate::online::cache::thumbs_online_cache_dir();
    let archive_cache = cache_dir.join("bing_archive.json");

    let raw_json = if archive_cache.exists() {
        tokio::fs::read_to_string(&archive_cache)
            .await
            .unwrap_or_default()
    } else {
        String::new()
    };

    let raw_json = if raw_json.len() > 1000 {
        raw_json
    } else {
        let archive_url = "https://raw.githubusercontent.com/zkeq/Bing-Wallpaper-Action/main/data/en-US_all.json";
        let resp = client
            .get(archive_url)
            .timeout(Duration::from_secs(15))
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64)")
            .send()
            .await
            .map_err(|e| format!("Failed to download Bing archive: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Bing archive server returned HTTP {}", resp.status()));
        }

        let body = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read archive body: {}", e))?;
        let _ = tokio::fs::write(&archive_cache, &body).await;
        body
    };

    let parsed: BingArchiveRoot = serde_json::from_str(&raw_json)
        .map_err(|e| format!("Failed to parse Bing archive: {}", e))?;

    let all_images = parsed.data.unwrap_or_default();
    let start_idx = ((page.saturating_sub(1)) as usize) * per_page;
    if start_idx >= all_images.len() {
        return Ok(Vec::new());
    }

    let end_idx = (start_idx + per_page).min(all_images.len());
    let slice = &all_images[start_idx..end_idx];

    let mut items = Vec::new();
    for img in slice.iter().cloned() {
        if let Some(item) = convert_bing_image(img) {
            items.push(item);
        }
    }

    Ok(items)
}

fn convert_bing_image(img: BingImage) -> Option<OnlineWallpaperItem> {
    let urlbase = img.urlbase?;
    let id = img.hsh.unwrap_or_else(|| {
        urlbase.trim_start_matches("/th?id=").to_string()
    });

    let title = img.title.filter(|t| !t.trim().is_empty()).unwrap_or_else(|| {
        if urlbase.contains("OHR.") {
            let part = urlbase.split("OHR.").nth(1).unwrap_or("");
            let core = part.split('_').next().unwrap_or(part);
            if !core.is_empty() {
                let mut s = String::new();
                let mut prev_is_lower = false;
                for c in core.chars() {
                    if prev_is_lower && c.is_uppercase() {
                        s.push(' ');
                    }
                    prev_is_lower = c.is_lowercase();
                    s.push(c);
                }
                if !s.is_empty() {
                    return s;
                }
            }
        }
        urlbase.split('.').nth(1).unwrap_or("Bing Wallpaper").replace('_', " ")
    });

    let copyright = img.copyright.unwrap_or_else(|| "Microsoft Bing".into());

    let formatted_date = img.startdate.and_then(|sd| {
        if sd.len() == 8 {
            Some(format!("{}-{}-{}", &sd[0..4], &sd[4..6], &sd[6..8]))
        } else {
            None
        }
    });

    let thumb_url = format!("https://www.bing.com{}_1920x1080.jpg&pid=hp&w=480&h=270&rs=1&c=4", urlbase);
    let full_url = format!("https://www.bing.com{}_UHD.jpg", urlbase);

    Some(OnlineWallpaperItem {
        id,
        title,
        author_or_copyright: copyright,
        thumb_url,
        full_url,
        resolution: "3840x2160 (4K UHD)".into(),
        source: OnlineSource::Bing,
        date: formatted_date,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_bing_image() {
        let raw = BingImage {
            urlbase: Some("/th?id=OHR.MountFuji_EN-US123".into()),
            copyright: Some("Mount Fuji in Japan (© Photographer/Getty)".into()),
            title: Some("Mount Fuji".into()),
            startdate: Some("20260904".into()),
            hsh: Some("abc123hash".into()),
        };

        let item = convert_bing_image(raw).expect("Conversion should succeed");
        assert_eq!(item.id, "abc123hash");
        assert_eq!(item.title, "Mount Fuji");
        assert_eq!(item.date, Some("2026-09-04".into()));
        assert_eq!(item.source, OnlineSource::Bing);
        assert!(item.full_url.contains("/th?id=OHR.MountFuji_EN-US123_UHD.jpg"));
        assert!(item.thumb_url.contains("480"));
    }

    #[test]
    fn test_parse_bing_archive_json() {
        let json_data = r#"{
            "Total": 1,
            "data": [
                {
                    "startdate": "20260904",
                    "urlbase": "/th?id=OHR.TestFuji_EN-US",
                    "copyright": "Test (© Getty)",
                    "title": "Fuji Wave",
                    "hsh": "xyz999"
                }
            ]
        }"#;

        let parsed: BingArchiveRoot = serde_json::from_str(json_data).expect("Should parse Bing archive JSON");
        assert!(parsed.data.is_some());
        let list = parsed.data.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].hsh.as_deref(), Some("xyz999"));
    }
}

