use std::collections::HashSet;
use std::time::Duration;
use super::models::{OnlineSource, OnlineWallpaperItem};

/// Fetches live wallpaper items from MotionBGS (motionbgs.com) with category, search, and pagination.
pub async fn fetch_motionbgs_wallpapers(
    client: &reqwest::Client,
    page: u32,
    query: Option<&str>,
    category: &str,
    resolution: &str,
) -> Result<Vec<OnlineWallpaperItem>, String> {
    let url = if let Some(q) = query {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            let encoded = form_urlencoded(trimmed);
            if page <= 1 {
                format!("https://motionbgs.com/search?q={}", encoded)
            } else {
                format!("https://motionbgs.com/search?q={}&page={}", encoded, page)
            }
        } else {
            build_catalog_url(category, page)
        }
    } else {
        build_catalog_url(category, page)
    };

    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(15))
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.5")
        .send()
        .await
        .map_err(|e| format!("Network request to MotionBGS failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("MotionBGS returned HTTP error code {}", resp.status()));
    }

    let html = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read MotionBGS response: {}", e))?;

    let items = parse_motionbgs_html(&html, resolution, category);
    if items.is_empty() {
        return Err("No live wallpapers found on MotionBGS for this query.".into());
    }

    Ok(items)
}

fn build_catalog_url(category: &str, page: u32) -> String {
    let cat = category.trim().to_lowercase();
    if cat.is_empty() || cat == "all" {
        if page <= 1 {
            "https://motionbgs.com/".to_string()
        } else {
            format!("https://motionbgs.com/{}/", page)
        }
    } else if page <= 1 {
        format!("https://motionbgs.com/tag:{}/", cat)
    } else {
        format!("https://motionbgs.com/tag:{}/{}/", cat, page)
    }
}

fn form_urlencoded(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}

pub fn parse_motionbgs_html(html: &str, preferred_res: &str, category: &str) -> Vec<OnlineWallpaperItem> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();

    // Skip <head> so <meta> tags are never accidentally parsed as wallpaper cards
    let html_slice = if let Some(idx) = html.find("<body").or_else(|| html.find("<div class=tmb").or_else(|| html.find("<div class=\"tmb\""))) {
        &html[idx..]
    } else {
        html
    };

    let mut pos = 0;

    let cat_label = if category.is_empty() || category == "all" {
        "Live".to_string()
    } else {
        let mut c = category.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    };

    while let Some(idx) = html_slice[pos..].find("/media/").map(|i| pos + i) {
        let a_idx = match html_slice[..idx].rfind("<a ") {
            Some(a) => a,
            None => {
                pos = idx + 7;
                continue;
            }
        };

        let after_media = idx + 7;
        let slash_idx = match html_slice[after_media..].find('/').map(|i| after_media + i) {
            Some(s) => s,
            None => {
                pos = idx + 7;
                continue;
            }
        };
        let media_id = &html_slice[after_media..slash_idx];

        let after_slash = slash_idx + 1;
        let jpg_idx = match html_slice[after_slash..].find(".jpg").map(|i| after_slash + i) {
            Some(j) if j - after_slash < 160 => j,
            _ => {
                pos = idx + 7;
                continue;
            }
        };
        let img_filename = &html_slice[after_slash..jpg_idx];

        if media_id.chars().all(|c| c.is_ascii_digit()) && !seen.contains(media_id) {
            seen.insert(media_id.to_string());

            // 1. Try extracting title from <a title="... live wallpaper">
            let mut title = if let Some(title_start) = html_slice[a_idx..idx].find("title=\"").map(|i| a_idx + i + 7) {
                if let Some(title_end) = html_slice[title_start..].find('"').map(|i| title_start + i) {
                    let raw_title = &html_slice[title_start..title_end];
                    let trimmed = if raw_title.to_lowercase().ends_with(" live wallpaper") {
                        &raw_title[..raw_title.len() - 15]
                    } else {
                        raw_title
                    };
                    html_decode(trimmed.trim())
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            // 2. If not found, try <span class=ttl>...</span> after idx
            if title.is_empty() {
                if let Some(span_pos) = html_slice[idx..].find("class=ttl>").or_else(|| html_slice[idx..].find("class=\"ttl\">")).map(|i| idx + i) {
                    let after_tag = if html_slice[span_pos..].starts_with("class=ttl>") {
                        span_pos + 10
                    } else {
                        span_pos + 12
                    };
                    if let Some(span_end) = html_slice[after_tag..].find('<').map(|i| after_tag + i) {
                        let raw = &html_slice[after_tag..span_end];
                        title = html_decode(raw.trim());
                    }
                }
            }

            // 3. If still empty, derive from img slug (e.g. "celestial-battle-gojo" -> "Celestial Battle Gojo")
            if title.is_empty() {
                let base_slug = img_filename.split('.').next().unwrap_or(img_filename);
                let words: Vec<String> = base_slug
                    .split('-')
                    .filter(|w| !w.is_empty() && !w.chars().all(|c| c.is_ascii_digit()))
                    .map(|w| {
                        let mut c = w.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        }
                    })
                    .collect();
                if !words.is_empty() {
                    title = words.join(" ");
                }
            }

            if title.is_empty() {
                title = format!("MotionBGS {}", media_id);
            }

            let is_4k = img_filename.to_lowercase().contains("4k") || img_filename.contains("3840x2160");
            let (resolution_str, full_url) = if preferred_res.eq_ignore_ascii_case("hd") {
                ("1080p HD", format!("https://motionbgs.com/dl/hd/{}/", media_id))
            } else if is_4k {
                ("4K UHD", format!("https://motionbgs.com/dl/4k/{}/", media_id))
            } else {
                ("1080p HD", format!("https://motionbgs.com/dl/hd/{}/", media_id))
            };

            let thumb_url = format!("https://motionbgs.com/i/c/364x205/media/{}/{}.jpg", media_id, img_filename);

            items.push(OnlineWallpaperItem {
                id: media_id.to_string(),
                title,
                author_or_copyright: format!("MotionBGS ({})", cat_label),
                thumb_url,
                full_url,
                resolution: resolution_str.to_string(),
                source: OnlineSource::MotionBGS,
                date: None,
            });
        }

        pos = jpg_idx + 4;
    }

    items
}

fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_motionbgs_html() {
        let sample_html = r#"
        <div class=tmb>
            <a title="Celestial Battle Gojo vs Mahoraga live wallpaper" href=/celestial-battle-gojo-vs-mahoraga>
                <figure><picture>
                    <img src=/i/c/364x205/media/9967/celestial-battle-gojo-vs-mahoraga.3840x2160.jpg width=364>
                </picture></figure>
                <span class=ttl>Celestial Battle Gojo vs Mahoraga</span>
                <span class=frm> 4K </span>
            </a>
            <a title="Dark Angel Rising live wallpaper" href=/dark-angel-rising>
                <figure><picture>
                    <img src=/i/c/364x205/media/10102/dark-angel-rising.3840x2160.jpg width=364>
                </picture></figure>
                <span class=ttl>Dark Angel Rising</span>
                <span class=frm> 4K </span>
            </a>
        </div>
        "#;

        let items = parse_motionbgs_html(sample_html, "4k", "anime");
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].id, "9967");
        assert_eq!(items[0].title, "Celestial Battle Gojo vs Mahoraga");
        assert_eq!(items[0].resolution, "4K UHD");
        assert_eq!(items[0].full_url, "https://motionbgs.com/dl/4k/9967/");
        assert_eq!(items[0].thumb_url, "https://motionbgs.com/i/c/364x205/media/9967/celestial-battle-gojo-vs-mahoraga.3840x2160.jpg");
        assert_eq!(items[0].author_or_copyright, "MotionBGS (Anime)");
        assert_eq!(items[0].source, OnlineSource::MotionBGS);

        assert_eq!(items[1].id, "10102");
        assert_eq!(items[1].title, "Dark Angel Rising");
        assert_eq!(items[1].full_url, "https://motionbgs.com/dl/4k/10102/");

        // Test 1080p resolution preference
        let items_hd = parse_motionbgs_html(sample_html, "hd", "anime");
        assert_eq!(items_hd[0].resolution, "1080p HD");
        assert_eq!(items_hd[0].full_url, "https://motionbgs.com/dl/hd/9967/");
    }

    #[test]
    fn test_build_catalog_url() {
        assert_eq!(build_catalog_url("all", 1), "https://motionbgs.com/");
        assert_eq!(build_catalog_url("all", 2), "https://motionbgs.com/2/");
        assert_eq!(build_catalog_url("anime", 1), "https://motionbgs.com/tag:anime/");
        assert_eq!(build_catalog_url("anime", 3), "https://motionbgs.com/tag:anime/3/");
    }

    #[test]
    fn test_form_urlencoded() {
        assert_eq!(form_urlencoded("gojo satoru"), "gojo+satoru");
        assert_eq!(form_urlencoded("cyberpunk 2077!"), "cyberpunk+2077%21");
    }
}
