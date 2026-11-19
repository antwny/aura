use std::collections::HashSet;
use std::time::Duration;

use super::models::{OnlineSource, OnlineWallpaperItem};

/// Lively Wallpaper's public MP4 library (livelywallpaper.app).
///
/// The site filters its catalogue client-side, so a text query is matched
/// against the title and the card's search metadata while paging.
pub async fn fetch_lively_wallpapers(
    client: &reqwest::Client,
    page: u32,
    query: Option<&str>,
) -> Result<Vec<OnlineWallpaperItem>, String> {
    let query = query.map(str::trim).filter(|q| !q.is_empty());

    // A query is not sent to the server: page through a few pages and filter
    // locally so unified search still finds something useful.
    let pages: Vec<u32> = if query.is_some() {
        (1..=3).collect()
    } else {
        vec![page.max(1)]
    };

    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let mut any_page = false;

    for p in pages {
        let url = if p <= 1 {
            "https://livelywallpaper.app/live-wallpapers/".to_string()
        } else {
            format!("https://livelywallpaper.app/live-wallpapers/{}/", p)
        };

        let html = client
            .get(&url)
            .timeout(Duration::from_secs(18))
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.5")
            .send()
            .await
            .map_err(|e| format!("Network request to Lively failed: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Lively returned an HTTP error: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read Lively response: {}", e))?;

        any_page = true;
        let parsed = parse_lively_html(&html, query);
        for item in parsed {
            if seen.insert(item.id.clone()) {
                items.push(item);
            }
        }

        if query.is_none() {
            break;
        }
    }

    if !any_page {
        return Err("Lively did not return a catalogue page.".into());
    }
    if items.is_empty() {
        return Err(match query {
            Some(q) => format!("No Lively wallpapers match \"{}\".", q),
            None => "Lively returned an empty catalogue.".into(),
        });
    }

    Ok(items)
}

pub fn parse_lively_html(html: &str, query: Option<&str>) -> Vec<OnlineWallpaperItem> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    let query = query.map(|q| q.to_lowercase());

    let mut rest = html;
    while let Some(start) = rest.find("<article class=\"wallpaper-card\"") {
        rest = &rest[start..];
        let end = match rest.find("</article>") {
            Some(e) => e,
            None => break,
        };
        let card = &rest[..end];

        if let Some(item) = parse_lively_card(card) {
            let matches = query.as_ref().is_none_or(|q| card.to_lowercase().contains(q));
            if matches && seen.insert(item.id.clone()) {
                items.push(item);
            }
        }

        rest = &rest[end + "</article>".len()..];
    }

    items
}

fn parse_lively_card(card: &str) -> Option<OnlineWallpaperItem> {
    let data = |key: &str| attr(card, key);
    let slug = data("data-preview-video")
        .and_then(|u| u.split("/wallpapers/").nth(1))
        .and_then(|u| u.split('/').next())
        .map(str::to_string)
        .or_else(|| {
            let href = attr_after(card, "class=\"wallpaper-card-media\" href=\"")?;
            let slug = href.trim_start_matches("./").trim_end_matches('/');
            (!slug.is_empty()).then(|| slug.to_string())
        })?;

    let title = data("data-title")
        .map(html_decode)
        .or_else(|| tag_text(card, "<h2>", "</h2>"))
        .unwrap_or_else(|| title_from_slug(&slug));
    if title.is_empty() {
        return None;
    }

    let thumb_url = attr_after(card, "<img src=\"")?.to_string();
    let thumb_url = if let Some(idx) = thumb_url.rfind("-640.") {
        format!("{}-1280.{}", &thumb_url[..idx], &thumb_url[idx + 5..])
    } else {
        thumb_url
    };

    let resolution = data("data-resolution")
        .map(|r| {
            let r = r.trim().to_uppercase();
            if r == "4K" {
                "4K UHD".to_string()
            } else {
                format!("{} HD", r.trim_end_matches('P'))
            }
        })
        .unwrap_or_else(|| "Live".to_string());

    Some(OnlineWallpaperItem {
        id: format!("lively-{}", slug),
        title,
        author_or_copyright: "Lively Wallpaper".to_string(),
        thumb_url,
        full_url: format!(
            "https://cdn.livelywallpaper.app/wallpapers/{}/download.mp4",
            slug
        ),
        resolution,
        source: OnlineSource::Lively,
        date: data("data-size").map(str::to_string),
    })
}

fn attr<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("{}=\"", key);
    attr_after(text, &pattern)
}

fn attr_after<'a>(text: &'a str, pattern: &str) -> Option<&'a str> {
    let start = text.find(pattern)? + pattern.len();
    let end = text[start..].find('"')? + start;
    Some(&text[start..end])
}

fn tag_text(text: &str, open: &str, close: &str) -> Option<String> {
    let start = text.find(open)? + open.len();
    let end = text[start..].find(close)? + start;
    Some(html_decode(text[start..end].trim()))
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

fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
    <article class="wallpaper-card" data-wallpaper-card data-category="games nature"
        data-title="Minecraft Aurora Night Cabin" data-resolution="4K" data-size="7.9 MB"
        data-preview-video="https://cdn.livelywallpaper.app/wallpapers/minecraft-aurora-night-cabin/preview.mp4">
        <a class="wallpaper-card-media" href="./minecraft-aurora-night-cabin/" aria-label="View">
            <img src="https://cdn.livelywallpaper.app/images/wallpapers/library/minecraft-aurora-night-cabin-640.webp">
        </a>
        <div class="wallpaper-card-body"><h2>Minecraft Aurora Night Cabin</h2></div>
    </article>
    <article class="wallpaper-card" data-title="Gojo Drinking Monster JJK" data-resolution="1080P"
        data-preview-video="https://cdn.livelywallpaper.app/wallpapers/gojo-drinking-monster-jjk/preview.mp4">
        <a class="wallpaper-card-media" href="./gojo-drinking-monster-jjk/"></a>
        <img src="https://cdn.livelywallpaper.app/images/wallpapers/library/gojo-drinking-monster-jjk-640.webp">
        <h2>Gojo Drinking Monster JJK</h2>
    </article>"#;

    #[test]
    fn parses_lively_cards() {
        let items = parse_lively_html(SAMPLE, None);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "lively-minecraft-aurora-night-cabin");
        assert_eq!(items[0].title, "Minecraft Aurora Night Cabin");
        assert_eq!(items[0].resolution, "4K UHD");
        assert_eq!(
            items[0].full_url,
            "https://cdn.livelywallpaper.app/wallpapers/minecraft-aurora-night-cabin/download.mp4"
        );
        assert_eq!(items[0].thumb_url, "https://cdn.livelywallpaper.app/images/wallpapers/library/minecraft-aurora-night-cabin-1280.webp");
        assert_eq!(items[1].resolution, "1080 HD");
    }

    #[test]
    fn filters_lively_query() {
        let items = parse_lively_html(SAMPLE, Some("gojo"));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Gojo Drinking Monster JJK");
    }
}
