use std::collections::HashSet;
use std::time::Duration;

use super::models::{OnlineSource, OnlineWallpaperItem};

/// DesktopHut (desktophut.com) live wallpapers.
///
/// Listing pages link to a detail page; the direct MP4 lives behind an
/// `id="primaryDownloadBtn"` anchor, so it is resolved at download time.
pub async fn fetch_desktophut_wallpapers(
    client: &reqwest::Client,
    page: u32,
    query: Option<&str>,
    category: &str,
) -> Result<Vec<OnlineWallpaperItem>, String> {
    let page = page.max(1);
    let url = if let Some(q) = query.map(str::trim).filter(|q| !q.is_empty()) {
        let encoded = form_urlencoded(q);
        if page <= 1 {
            format!("https://www.desktophut.com/search/{}", encoded)
        } else {
            format!("https://www.desktophut.com/search/{}?page={}", encoded, page)
        }
    } else {
        let base = if category.is_empty() || category == "all" {
            "https://www.desktophut.com/".to_string()
        } else {
            format!("https://www.desktophut.com/category/{}", category)
        };
        if page <= 1 {
            base
        } else {
            format!("{}?page={}", base, page)
        }
    };

    let html = client
        .get(&url)
        .timeout(Duration::from_secs(18))
        .header(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0",
        )
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .header("Accept-Language", "en-US,en;q=0.5")
        .send()
        .await
        .map_err(|e| format!("Network request to DesktopHut failed: {}", e))?
        .error_for_status()
        .map_err(|e| format!("DesktopHut returned an HTTP error: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read DesktopHut response: {}", e))?;

    let items = parse_desktophut_html(&html);
    if items.is_empty() {
        return Err(match query {
            Some(q) => format!("No DesktopHut wallpapers match \"{}\".", q),
            None => "DesktopHut returned an empty catalogue.".into(),
        });
    }
    Ok(items)
}

pub fn parse_desktophut_html(html: &str) -> Vec<OnlineWallpaperItem> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();

    let mut rest = html;
    while let Some(start) = rest.find("class=\"wallpaper-card\"") {
        // Walk back to the opening anchor.
        let anchor_start = match rest[..start].rfind("<a ") {
            Some(i) => i,
            None => {
                rest = &rest[start + "class=\"wallpaper-card\"".len()..];
                continue;
            }
        };
        let block_start = anchor_start;
        let block = &rest[block_start..];
        let end = match block.find("</a>") {
            Some(e) => e,
            None => break,
        };
        let card = &block[..end + 4];

        if let Some(item) = parse_desktophut_card(card) {
            if seen.insert(item.id.clone()) {
                items.push(item);
            }
        }

        rest = &block[end + 4..];
    }

    items
}

fn parse_desktophut_card(card: &str) -> Option<OnlineWallpaperItem> {
    let href = attr(card, "href")?;
    if !href.starts_with('/') {
        return None;
    }
    // Skip category or pagination links that happen to share the class.
    if href.starts_with("/category/")
        || href.starts_with("/search/")
        || href.starts_with("/page/")
        || href.starts_with("/?")
    {
        return None;
    }

    let slug = href.trim_start_matches('/').trim_end_matches('/');
    if slug.is_empty() || slug.contains('/') {
        return None;
    }

    let img = attr_after(card, "<img src=\"")?;
    if img.is_empty() {
        return None;
    }

    // DesktopHut tags WEB / Scene cards; those are not plain MP4 loops.
    if card.contains("badge-web") || card.contains("badge-scene") {
        let lower = card.to_lowercase();
        if lower.contains(">web<") || lower.contains(">scene<") {
            return None;
        }
    }

    let title = tag_text(card, "class=\"card-title\">", "</h3>")
        .or_else(|| attr(card, "title").map(clean_desktophut_title))
        .or_else(|| attr(card, "aria-label").map(str::to_string))
        .map(|t| html_decode(t.trim()))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| title_from_slug(slug));

    let resolution = extract_resolution(card).unwrap_or_else(|| "Live".to_string());

    let page_url = if href.starts_with("http") {
        href.to_string()
    } else {
        format!("https://www.desktophut.com{}", href)
    };

    Some(OnlineWallpaperItem {
        id: format!("dh-{}", slug),
        title,
        author_or_copyright: "DesktopHut".to_string(),
        thumb_url: img.to_string(),
        full_url: page_url,
        resolution,
        source: OnlineSource::DesktopHut,
        date: None,
    })
}

/// Extracts the MP4 URL from a DesktopHut detail page.
///
/// Most wallpapers expose an MP4 behind `primaryDownloadBtn`. Some entries are
/// Windows-only packages (`.zip`/`.web`) with no direct MP4; for those we fall
/// back to the page's preview player, which is still a loopable video.
pub fn parse_desktophut_download(html: &str) -> Option<String> {
    parse_desktophut_primary(html).or_else(|| parse_desktophut_preview(html))
}

fn parse_desktophut_primary(html: &str) -> Option<String> {
    let marker = "id=\"primaryDownloadBtn\"";
    let idx = html.find(marker)?;
    // The anchor's href can appear before or after the id depending on markup.
    let window_start = idx.saturating_sub(1200);
    let window_end = (idx + 1200).min(html.len());
    let window = &html[window_start..window_end];

    let mut candidates = Vec::new();
    let mut search = window;
    while let Some(i) = search.find("href=\"") {
        let after = &search[i + 6..];
        let end = match after.find('"') {
            Some(e) => e,
            None => break,
        };
        let value = &after[..end];
        if value.contains(".mp4") || value.contains(".webm") {
            candidates.push(value.to_string());
        }
        search = &after[end..];
    }

    let value = candidates
        .into_iter()
        .find(|c| !c.to_lowercase().contains("preview"))?;

    if value.starts_with("http") {
        Some(value)
    } else if value.starts_with('/') {
        Some(format!("https://www.desktophut.com{}", value))
    } else {
        Some(format!("https://www.desktophut.com/{}", value))
    }
}

/// Some DesktopHut entries ship a Windows package instead of a video file.
/// Their detail page still embeds a preview player we can use instead.
pub fn parse_desktophut_preview(html: &str) -> Option<String> {
    let mut rest = html;
    while let Some(i) = rest.find("<source src=\"") {
        let start = i + "<source src=\"".len();
        let end = rest[start..].find('"')? + start;
        let value = &rest[start..end];
        if value.contains("/previews/") && (value.contains(".mp4") || value.contains(".webm")) {
            return Some(if value.starts_with("http") {
                value.to_string()
            } else if value.starts_with('/') {
                format!("https://www.desktophut.com{}", value)
            } else {
                format!("https://www.desktophut.com/{}", value)
            });
        }
        rest = &rest[start..];
    }
    None
}

fn extract_resolution(card: &str) -> Option<String> {
    let lower = card.to_lowercase();
    if lower.contains(">4k<") || lower.contains(" 4k ") || lower.contains("(4k") {
        return Some("4K".to_string());
    }
    // Look for 3840x2160 / 1920x1080 style labels.
    let mut idx = 0;
    while let Some(pos) = lower[idx..].find('x') {
        let at = idx + pos;
        let digits_before = lower[..at]
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .count();
        if digits_before >= 3 {
            let start = at - digits_before;
            let height: String = lower[at + 1..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if height.len() >= 3 {
                let width = &lower[start..at];
                return Some(format!("{}x{}", width, height));
            }
        }
        idx = at + 1;
    }
    None
}

fn clean_desktophut_title(raw: &str) -> String {
    raw.replace("Download ", "")
        .replace(" live wallpaper (4K/HD)", "")
        .replace(" Live Wallpaper", "")
        .trim()
        .to_string()
}

fn attr<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("{}=\"", key);
    let start = text.find(&pattern)? + pattern.len();
    let end = text[start..].find('"')? + start;
    Some(&text[start..end])
}

fn attr_after<'a>(text: &'a str, pattern: &str) -> Option<&'a str> {
    let start = text.find(pattern)? + pattern.len();
    let end = text[start..].find('"')? + start;
    Some(&text[start..end])
}

fn tag_text(text: &str, open: &str, close: &str) -> Option<String> {
    let start = text.find(open)? + open.len();
    let end = text[start..].find(close)? + start;
    let value = strip_tags(&text[start..end]);
    let value = value.trim();
    (!value.is_empty()).then(|| html_decode(value))
}

fn strip_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

fn title_from_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|w| !w.is_empty() && !w.chars().all(|c| c.is_ascii_digit()))
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn form_urlencoded(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = r#"
    <a href="/sung-jinwoo-solo-leveling-live-wallpaper-ecz8" class="wallpaper-card" title="Download Sung Jinwoo Solo Leveling Live Wallpaper live wallpaper (4K/HD)" aria-label="Sung Jinwoo Solo Leveling Live Wallpaper">
        <img src="https://www.desktophut.com/images/7SbyR0cQtL-sung-jinwoo-solo-leveling-live-wallpaper.webp" alt="x">
        <div class="card-badges"><span>4K</span></div>
        <h3 class="card-title">Sung Jinwoo Solo Leveling</h3>
    </a>
    <a href="/category/anime-live-wallpapers" class="wallpaper-card"><h3 class="card-title">Anime</h3></a>
    "#;

    const DETAIL: &str = r#"
    <a href="https://www.desktophut.com/files/7SbyR0cQtL-sung-jinwoo-solo-leveling-live-wallpaper.mp4"
       download="sung-jinwoo-solo-leveling-live-wallpaper.mp4" rel="nofollow" id="primaryDownloadBtn" class="hover-yellow-btn">
    "#;

    const DETAIL_PACKAGE: &str = r#"
    <video poster="https://www.desktophut.com/images/abc.webp">
        <source src="https://www.desktophut.com/previews/abc-player.mp4" type="video/mp4">
    </video>
    <a href="https://www.desktophut.com/files/abc-wallpaper.zip" download="abc-wallpaper.zip"
       rel="nofollow" id="primaryDownloadBtn" class="hover-yellow-btn">
    "#;

    #[test]
    fn parses_listing() {
        let items = parse_desktophut_html(LISTING);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "dh-sung-jinwoo-solo-leveling-live-wallpaper-ecz8");
        assert_eq!(items[0].resolution, "4K");
        assert_eq!(
            items[0].full_url,
            "https://www.desktophut.com/sung-jinwoo-solo-leveling-live-wallpaper-ecz8"
        );
    }

    #[test]
    fn listing_urls_keep_the_origin_on_paginated_pages() {
        let items = parse_desktophut_html(LISTING);
        assert!(!items[0].full_url.contains("?page="));
        assert!(items[0].full_url.starts_with("https://www.desktophut.com/"));
    }

    #[test]
    fn extracts_download() {
        assert_eq!(
            parse_desktophut_download(DETAIL).as_deref(),
            Some("https://www.desktophut.com/files/7SbyR0cQtL-sung-jinwoo-solo-leveling-live-wallpaper.mp4")
        );
    }

    #[test]
    fn falls_back_to_preview_for_packages() {
        assert_eq!(
            parse_desktophut_download(DETAIL_PACKAGE).as_deref(),
            Some("https://www.desktophut.com/previews/abc-player.mp4")
        );
    }
}
