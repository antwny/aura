use std::collections::HashSet;
use std::time::Duration;

use super::models::{OnlineSource, OnlineWallpaperItem};

/// MoeWalls (moewalls.com) live wallpapers.
///
/// The catalogue stores the wallpaper page URL; the detail page exposes a
/// tokenised `moe-download` anchor that resolves to a direct MP4 through
/// `go.moewalls.com`.
pub async fn fetch_moewalls_wallpapers(
    client: &reqwest::Client,
    page: u32,
    query: Option<&str>,
    category: &str,
) -> Result<Vec<OnlineWallpaperItem>, String> {
    let page = page.max(1);
    let url = if let Some(q) = query.map(str::trim).filter(|q| !q.is_empty()) {
        let encoded = form_urlencoded(q);
        if page <= 1 {
            format!("https://moewalls.com/?s={}", encoded)
        } else {
            format!("https://moewalls.com/page/{}/?s={}", page, encoded)
        }
    } else {
        let base = if category.is_empty() || category == "all" {
            "https://moewalls.com/".to_string()
        } else {
            format!("https://moewalls.com/category/{}/", category)
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
        .map_err(|e| format!("Network request to MoeWalls failed: {}", e))?
        .error_for_status()
        .map_err(|e| format!("MoeWalls returned an HTTP error: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read MoeWalls response: {}", e))?;

    let items = parse_moewalls_html(&html);
    if items.is_empty() {
        return Err(match query {
            Some(q) => format!("No MoeWalls wallpapers match \"{}\".", q),
            None => "MoeWalls returned an empty catalogue.".into(),
        });
    }
    Ok(items)
}

pub fn parse_moewalls_html(html: &str) -> Vec<OnlineWallpaperItem> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();

    let mut rest = html;
    while let Some(start) = rest.find("<article class=\"entry-tpl-grid") {
        rest = &rest[start..];
        let end = match rest.find("</article>") {
            Some(e) => e,
            None => break,
        };
        let card = &rest[..end + "</article>".len()];

        if let Some(item) = parse_moewalls_card(card) {
            if seen.insert(item.id.clone()) {
                items.push(item);
            }
        }

        rest = &rest[end + "</article>".len()..];
    }

    items
}

fn parse_moewalls_card(card: &str) -> Option<OnlineWallpaperItem> {
    // The anchor attributes are not in a fixed order across site updates.
    let frame_start = card.find("class=\"g1-frame\"")?;
    let href = attr_after(&card[frame_start..], "href=\"")?;
    if !href.contains("moewalls.com") {
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

    let title = attr_after(card, "title=\"")
        .map(clean_title)
        .filter(|t| !t.is_empty())
        .or_else(|| attr_after(card, "<img alt=\"").map(clean_title))
        .or_else(|| tag_text(card, "class=\"entry-title\">", "</h"))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| title_from_slug(slug));

    let img_start = card.find("<img")?;
    let img = attr_after(&card[img_start..], "src=\"")?;
    let thumb_url = if img.starts_with("http") {
        img.to_string()
    } else {
        format!("https://moewalls.com{}", img)
    };

    let resolution = resolution_from_class(card).unwrap_or_else(|| "Live".to_string());

    Some(OnlineWallpaperItem {
        id: format!("mw-{}-{}", category, slug),
        title,
        author_or_copyright: "MoeWalls".to_string(),
        thumb_url,
        full_url: format!("https://moewalls.com/{}/{}/", category, slug),
        resolution,
        source: OnlineSource::MoeWalls,
        date: None,
    })
}

/// Extracts the tokenised download URL from a MoeWalls detail page.
///
/// The token is already URL-encoded (`%2F`, `%2B`) and must be concatenated
/// verbatim; decoding it would break the signed query string.
pub fn parse_moewalls_download(html: &str) -> Option<String> {
    let start = html.find("id=\"moe-download\"")?;
    let token = attr_after(&html[start..], "data-url=\"")?;
    if token.is_empty() {
        return None;
    }
    Some(format!(
        "https://go.moewalls.com./download.php?video={}",
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
        Some(value.replace('x', "x"))
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

fn tag_text(text: &str, open: &str, close: &str) -> Option<String> {
    let start = text.find(open)? + open.len();
    let end = text[start..].find(close)? + start;
    let value = text[start..end].trim();
    (!value.is_empty()).then(|| html_decode(value))
}

fn title_from_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|w| !w.is_empty())
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
    <article class="entry-tpl-grid entry-tpl-grid-m g1-card post-21754 post type-post status-publish resolutions-3840x2160">
        <a class="g1-frame" href="https://moewalls.com/movies/spiderman-tears-in-the-rain-live-wallpaper/" title="SpiderMan Tears In The Rain Live Wallpaper">
            <img src="https://moewalls.com/wp-content/uploads/2026/01/spiderman-thumb.jpg" alt="SpiderMan Tears In The Rain">
        </a>
    </article>"#;

    const DOWNLOAD_SAMPLE: &str = r##"
    <a id="moe-download" data-id ="21734" data-url="y1IF7HBibYh%2BTtHe4Gil2NsA%2F1U5fFnX%2FILBhTjC7IrSZ%2FWVgk%2F" rel="nofollow" class="g1-button lcc-wall" href="#">
    "##;

    #[test]
    fn parses_moewalls_download_token() {
        let url = parse_moewalls_download(DOWNLOAD_SAMPLE).expect("download url");
        assert_eq!(
            url,
            "https://go.moewalls.com./download.php?video=y1IF7HBibYh%2BTtHe4Gil2NsA%2F1U5fFnX%2FILBhTjC7IrSZ%2FWVgk%2F"
        );
    }

    #[test]
    fn parses_moewalls_card() {
        let items = parse_moewalls_html(SAMPLE);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "mw-movies-spiderman-tears-in-the-rain-live-wallpaper");
        assert_eq!(items[0].title, "SpiderMan Tears In The Rain");
        assert_eq!(items[0].resolution, "3840x2160");
        assert_eq!(
            items[0].full_url,
            "https://moewalls.com/movies/spiderman-tears-in-the-rain-live-wallpaper/"
        );
        assert!(items[0]
            .thumb_url
            .starts_with("https://moewalls.com/wp-content/uploads/"));
    }
}
