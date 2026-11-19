use std::collections::HashSet;
use std::time::Duration;

use super::models::{OnlineSource, OnlineWallpaperItem};

/// WallpaperWaifu (wallpaperwaifu.com) live wallpapers.
///
/// The catalogue stores the wallpaper page URL; the detail page exposes a
/// tokenised `wf-download` anchor that resolves to a direct MP4 through
/// `go.wallpaperwaifu.com`.
pub async fn fetch_wallpaperwaifu_wallpapers(
    client: &reqwest::Client,
    page: u32,
    query: Option<&str>,
    category: &str,
) -> Result<Vec<OnlineWallpaperItem>, String> {
    let page = page.max(1);
    let url = if let Some(q) = query.map(str::trim).filter(|q| !q.is_empty()) {
        let encoded = form_urlencoded(q);
        if page <= 1 {
            format!("https://wallpaperwaifu.com/?s={}", encoded)
        } else {
            format!("https://wallpaperwaifu.com/page/{}/?s={}", page, encoded)
        }
    } else {
        let base = if category.is_empty() || category == "all" {
            "https://wallpaperwaifu.com/".to_string()
        } else {
            format!("https://wallpaperwaifu.com/category/{}/", category)
        };
        if page <= 1 {
            base
        } else {
            format!("{}page/{}/", base, page)
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
        .map_err(|e| format!("Network request to WallpaperWaifu failed: {}", e))?
        .error_for_status()
        .map_err(|e| format!("WallpaperWaifu returned an HTTP error: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read WallpaperWaifu response: {}", e))?;

    let items = parse_wallpaperwaifu_html(&html);
    if items.is_empty() {
        return Err(match query {
            Some(q) => format!("No WallpaperWaifu wallpapers match \"{}\".", q),
            None => "WallpaperWaifu returned an empty catalogue.".into(),
        });
    }
    Ok(items)
}

pub fn parse_wallpaperwaifu_html(html: &str) -> Vec<OnlineWallpaperItem> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();

    let mut rest = html;
    while let Some(start) = rest.find("<article data-id=\"post-") {
        rest = &rest[start..];
        let end = match rest.find("</article>") {
            Some(e) => e,
            None => break,
        };
        let card = &rest[..end + "</article>".len()];

        if let Some(item) = parse_wallpaperwaifu_card(card) {
            if seen.insert(item.id.clone()) {
                items.push(item);
            }
        }

        rest = &rest[end + "</article>".len()..];
    }

    items
}

fn parse_wallpaperwaifu_card(card: &str) -> Option<OnlineWallpaperItem> {
    let href = attr_after(card, "<a href=\"")?;
    if !href.starts_with("https://wallpaperwaifu.com/") || href.contains("/category/") {
        return None;
    }
    let slug = href
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())?;
    let category = href
        .trim_end_matches('/')
        .rsplit('/')
        .nth(1)
        .filter(|s| !s.is_empty())
        .unwrap_or("wallpaper");

    let title = attr_after(&card[..], "title=\"")
        .map(clean_title)
        .filter(|t| !t.is_empty())
        .or_else(|| attr_after(card, "alt=\"").map(clean_title))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| title_from_slug(slug));

    let thumb_url = attr_after(card, "data-src=\"")
        .or_else(|| attr_after(card, "<img src=\""))
        .map(|s| {
            if s.starts_with("http") || s.starts_with("//") {
                s.to_string()
            } else {
                format!("https://wallpaperwaifu.com{}", s)
            }
        })?;

    let resolution = resolution_from_class(card).unwrap_or_else(|| "Live".to_string());

    Some(OnlineWallpaperItem {
        id: format!("ww-{}-{}", category, slug),
        title,
        author_or_copyright: "WallpaperWaifu".to_string(),
        thumb_url,
        full_url: format!("https://wallpaperwaifu.com/{}/{}/", category, slug),
        resolution,
        source: OnlineSource::WallpaperWaifu,
        date: None,
    })
}

/// Extracts the tokenised download URL from a WallpaperWaifu detail page.
///
/// The token is URL-encoded and must be concatenated verbatim.
pub fn parse_wallpaperwaifu_download(html: &str) -> Option<String> {
    let start = html.find("id=\"wf-download\"")?;
    let token = attr_after(&html[start..], "data-url=\"")?;
    if token.is_empty() {
        return None;
    }
    Some(format!(
        "https://go.wallpaperwaifu.com./download.php?video={}",
        token
    ))
}

fn resolution_from_class(card: &str) -> Option<String> {
    let marker = "resolutions-";
    let start = card.find(marker)? + marker.len();
    let value: String = card[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == 'x')
        .collect();
    if value.contains('x') && value.len() >= 8 {
        Some(value)
    } else {
        None
    }
}

fn clean_title(raw: &str) -> String {
    html_decode(raw)
        .replace(" Live Wallpaper", "")
        .replace(" live wallpaper", "")
        .trim()
        .to_string()
}

fn attr_after<'a>(text: &'a str, pattern: &str) -> Option<&'a str> {
    let start = text.find(pattern)? + pattern.len();
    let end = text[start..].find('"')? + start;
    Some(&text[start..end])
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

    const SAMPLE: &str = r#"
    <article data-id="post-9511" class="wall-grid-item post-9511 post type-post status-publish resolutions-3840x2160">
        <a href="https://wallpaperwaifu.com/anime/sad-day-sailor-moon-live-wallpaper-2847/" title="Sad Day Sailor Moon Live Wallpaper">
            <img data-src="https://wallpaperwaifu.com/wp-content/uploads/2024/11/sad-day-sailor-moon-thumb-350x233.jpg" alt="Sad Day Sailor Moon">
        </a>
    </article>"#;

    const DOWNLOAD_SAMPLE: &str = r##"
    <a href="#" id="wf-download" data-id ="9466" data-url="3k9ktoiPUhLunKDeCvtz2WRMD8V8sbSwh8y%2Fhio5kEdo9Q8gjck6MtImfMXsqai%2BJ7gveZmxlXRgPsws" rel="nofollow noopener sponsored" class="btndownload">
    "##;

    #[test]
    fn parses_waifu_download_token() {
        let url = parse_wallpaperwaifu_download(DOWNLOAD_SAMPLE).expect("download url");
        assert_eq!(
            url,
            "https://go.wallpaperwaifu.com./download.php?video=3k9ktoiPUhLunKDeCvtz2WRMD8V8sbSwh8y%2Fhio5kEdo9Q8gjck6MtImfMXsqai%2BJ7gveZmxlXRgPsws"
        );
    }

    #[test]
    fn parses_waifu_card() {
        let items = parse_wallpaperwaifu_html(SAMPLE);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "ww-anime-sad-day-sailor-moon-live-wallpaper-2847");
        assert_eq!(items[0].title, "Sad Day Sailor Moon");
        assert_eq!(items[0].resolution, "3840x2160");
        assert_eq!(
            items[0].full_url,
            "https://wallpaperwaifu.com/anime/sad-day-sailor-moon-live-wallpaper-2847/"
        );
    }
}
