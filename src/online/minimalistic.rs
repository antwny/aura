use std::time::Duration;
use super::models::{OnlineSource, OnlineWallpaperItem};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MinimalRawEntry {
    pub name: String,
    pub path: Option<String>,
    pub download_url: String,
    pub size: Option<u64>,
}

pub fn parse_minimal_filename(filename: &str) -> (String, String) {
    let stem = std::path::Path::new(filename)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| filename.to_string());

    let (author_raw, title_raw) = if stem.to_lowercase().starts_with("alena-aenami-") {
        ("Alena Aenami", &stem["alena-aenami-".len()..])
    } else if stem.to_lowercase().starts_with("unknown-") {
        ("", &stem["unknown-".len()..])
    } else if let Some((a, t)) = stem.split_once('-') {
        (a, t)
    } else {
        ("", stem.as_str())
    };

    let author = if author_raw.is_empty() {
        "Minimalist".to_string()
    } else if author_raw == "Alena Aenami" {
        "Alena Aenami".to_string()
    } else {
        capitalize_words(&author_raw.replace('_', " "))
    };

    let title = capitalize_words(&title_raw.replace(['-', '_'], " "));
    (title, author)
}

fn capitalize_words(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub async fn fetch_minimalistic_wallpapers(
    client: &reqwest::Client,
) -> Result<Vec<OnlineWallpaperItem>, String> {
    let catalog_url = "https://raw.githubusercontent.com/DenverCoder1/minimalistic-wallpaper-collection/main/api/generated/images.json";
    let cache_dir = crate::online::cache::thumbs_online_cache_dir();
    let cache_file = cache_dir.join("minimalistic_catalog.json");

    // Check if cache file exists and is less than 24h old
    if cache_file.exists() {
        if let Ok(metadata) = cache_file.metadata() {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    if elapsed < Duration::from_secs(86400) {
                        if let Ok(content) = tokio::fs::read_to_string(&cache_file).await {
                            if let Ok(items) = parse_minimalistic_json(&content) {
                                if !items.is_empty() {
                                    return Ok(items);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Fetch from GitHub raw
    let resp_res = client
        .get(catalog_url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64; Aura)")
        .timeout(Duration::from_secs(15))
        .send()
        .await;

    match resp_res {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.text().await {
                    Ok(text) => {
                        let _ = tokio::fs::write(&cache_file, &text).await;
                        parse_minimalistic_json(&text)
                    }
                    Err(e) => {
                        // Fallback to cache if available
                        if cache_file.exists() {
                            if let Ok(content) = tokio::fs::read_to_string(&cache_file).await {
                                if let Ok(items) = parse_minimalistic_json(&content) {
                                    return Ok(items);
                                }
                            }
                        }
                        Err(format!("Error reading response text: {}", e))
                    }
                }
            } else {
                if cache_file.exists() {
                    if let Ok(content) = tokio::fs::read_to_string(&cache_file).await {
                        if let Ok(items) = parse_minimalistic_json(&content) {
                            return Ok(items);
                        }
                    }
                }
                Err(format!("HTTP error: {}", resp.status()))
            }
        }
        Err(e) => {
            if cache_file.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&cache_file).await {
                    if let Ok(items) = parse_minimalistic_json(&content) {
                        return Ok(items);
                    }
                }
            }
            Err(format!("Connection error: {}", e))
        }
    }
}

pub fn parse_minimalistic_json(json_str: &str) -> Result<Vec<OnlineWallpaperItem>, String> {
    let entries: Vec<MinimalRawEntry> = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse minimalistic JSON: {}", e))?;

    let mut items = Vec::with_capacity(entries.len());
    for entry in entries {
        let (title, author) = parse_minimal_filename(&entry.name);
        let resolution = if let Some(sz) = entry.size {
            if sz > 2_000_000 {
                "4K UHD".to_string()
            } else if sz > 800_000 {
                "2K QHD".to_string()
            } else {
                "1080p HD".to_string()
            }
        } else {
            "HD".to_string()
        };

        let full_url = entry.download_url.clone();
        let thumb_url = format!(
            "https://wsrv.nl/?url={}&w=400&h=225&fit=cover&output=jpg&q=80",
            entry.download_url
        );

        items.push(OnlineWallpaperItem {
            id: format!("minimal_{}", entry.name.replace('.', "_")),
            title,
            author_or_copyright: author,
            thumb_url,
            full_url,
            resolution,
            source: OnlineSource::Minimalistic,
            date: None,
        });
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_filename() {
        let (title, author) = parse_minimal_filename("alena-aenami-clouds.jpg");
        assert_eq!(title, "Clouds");
        assert_eq!(author, "Alena Aenami");

        let (title2, author2) = parse_minimal_filename("OGARart-eagle-mountain-sunset-minimalist.jpg");
        assert_eq!(title2, "Eagle Mountain Sunset Minimalist");
        assert_eq!(author2, "OGARart");

        let (title3, author3) = parse_minimal_filename("Electronic_Sample_96-calm-night.png");
        assert_eq!(title3, "Calm Night");
        assert_eq!(author3, "Electronic Sample 96");

        let (title4, author4) = parse_minimal_filename("unknown-forest-dawn.jpg");
        assert_eq!(title4, "Forest Dawn");
        assert_eq!(author4, "Minimalist");
    }

    #[test]
    fn test_parse_minimalistic_json() {
        let json = r#"[
            {
                "name": "alena-aenami-clouds.jpg",
                "path": "images/alena-aenami-clouds.jpg",
                "download_url": "https://raw.githubusercontent.com/DenverCoder1/minimalistic-wallpaper-collection/main/images/alena-aenami-clouds.jpg",
                "size": 828770
            }
        ]"#;

        let items = parse_minimalistic_json(json).expect("should parse");
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item.source, OnlineSource::Minimalistic);
        assert_eq!(item.title, "Clouds");
        assert_eq!(item.author_or_copyright, "Alena Aenami");
        assert_eq!(item.resolution, "2K QHD");
        assert_eq!(item.id, "minimal_alena-aenami-clouds_jpg");
    }
}
