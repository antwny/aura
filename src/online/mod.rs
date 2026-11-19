pub mod models;
pub mod filters;
pub mod cache;
pub mod motionbgs;
pub mod livewallpapers4free;
pub mod lively;
pub mod desktophut;
pub mod moewalls;
pub mod wallpaperwaifu;
pub mod bing;
pub mod wallhaven;
pub mod minimalistic;
pub mod updater;

pub use models::{OnlineSource, OnlineWallpaperItem};
pub use cache::{download_online_thumbnail, download_to_file_with_progress, wallpapers_online_dir};
pub use motionbgs::fetch_motionbgs_wallpapers;
pub use livewallpapers4free::{fetch_livewallpapers4free_page, LiveWallpapers4FreePage};
pub use lively::fetch_lively_wallpapers;
pub use desktophut::{fetch_desktophut_wallpapers, parse_desktophut_download};
pub use moewalls::{fetch_moewalls_wallpapers, parse_moewalls_download};
pub use wallpaperwaifu::{fetch_wallpaperwaifu_wallpapers, parse_wallpaperwaifu_download};
pub use bing::{fetch_bing_wallpapers, fetch_bing_archive_page};
pub use wallhaven::fetch_wallhaven_wallpapers;
pub use minimalistic::fetch_minimalistic_wallpapers;

use filters::{category_slug, resolution_matches, resolution_slug, CategoryFilter, ResolutionFilter};
use std::collections::HashSet;

/// Result of a unified search across every catalogue.
#[derive(Debug, Default, Clone)]
pub struct UnifiedSearchPage {
    /// Interleaved results so no single catalogue dominates the first row.
    pub items: Vec<OnlineWallpaperItem>,
    /// Human readable failures from catalogues that could not be reached.
    pub errors: Vec<String>,
    /// Whether at least one catalogue may have another page.
    pub has_more: bool,
    /// Catalogues left out because they cannot honour the active category.
    pub skipped: Vec<OnlineSource>,
}

/// Search every enabled catalogue concurrently and interleave the results.
///
/// Individual catalogue failures are reported through [`UnifiedSearchPage::errors`]
/// instead of failing the whole request; only a total failure is an error.
pub async fn fetch_unified_wallpapers(
    client: &reqwest::Client,
    page: u32,
    query: Option<&str>,
    filter: Option<OnlineSource>,
    category: Option<CategoryFilter>,
    resolution: Option<ResolutionFilter>,
) -> Result<UnifiedSearchPage, String> {
    let page = page.max(1);
    let query = query.map(str::trim).filter(|q| !q.is_empty());
    let requested: Vec<OnlineSource> = match filter {
        Some(source) => vec![source],
        None => OnlineSource::catalogs().to_vec(),
    };

    // Sources that cannot honour the active category are skipped rather than
    // returning unrelated results.
    let mut skipped = Vec::new();
    let mut sources = Vec::new();
    for source in requested {
        if let Some(category) = category {
            if category_slug(source, category).is_none() {
                skipped.push(source);
                continue;
            }
        }
        sources.push(source);
    }

    let futures = sources.into_iter().map(|source| async move {
        // One slow catalogue must not hold the whole unified grid hostage.
        let result = match tokio::time::timeout(
            std::time::Duration::from_secs(8),
            fetch_from_source(client, source, page, query, category, resolution),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err("timed out after 8s".to_string()),
        };
        (source, result)
    });

    let results = futures::future::join_all(futures).await;

    let mut per_source: Vec<(OnlineSource, Vec<OnlineWallpaperItem>)> = Vec::new();
    let mut errors = Vec::new();
    let mut has_more = false;

    for (source, result) in results {
        match result {
            Ok((mut items, more)) => {
                if let Some(resolution) = resolution {
                    items.retain(|item| resolution_matches(&item.resolution, resolution));
                }
                has_more |= more && !items.is_empty();
                if !items.is_empty() {
                    per_source.push((source, items));
                }
            }
            Err(error) => errors.push(format!("{}: {}", source.label(), error)),
        }
    }

    if per_source.is_empty() {
        if let Some(first) = errors.first() {
            return Err(first.clone());
        }
        return Ok(UnifiedSearchPage {
            items: Vec::new(),
            errors,
            has_more: false,
            skipped,
        });
    }

    // Round-robin: one item from each catalogue per pass, capped at the
    // catalogue page size so no item is skipped when "Load more" advances.
    const PER_SOURCE_CAP: usize = 24;
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let mut index = 0;
    loop {
        let mut pushed = false;
        for (_, source_items) in per_source.iter_mut() {
            if index >= PER_SOURCE_CAP {
                break;
            }
            if let Some(item) = source_items.get(index) {
                if seen.insert(item.id.clone()) {
                    items.push(item.clone());
                    pushed = true;
                }
            }
        }
        if !pushed || index + 1 >= PER_SOURCE_CAP {
            break;
        }
        index += 1;
    }

    Ok(UnifiedSearchPage {
        items,
        errors,
        has_more,
        skipped,
    })
}

async fn fetch_from_source(
    client: &reqwest::Client,
    source: OnlineSource,
    page: u32,
    query: Option<&str>,
    category: Option<CategoryFilter>,
    resolution: Option<ResolutionFilter>,
) -> Result<(Vec<OnlineWallpaperItem>, bool), String> {
    let category_value = category.and_then(|category| category_slug(source, category));
    let resolution_value = resolution.and_then(|resolution| resolution_slug(source, resolution));
    match source {
        OnlineSource::Lively => {
            let items = fetch_lively_wallpapers(client, page, query).await?;
            Ok((items, true))
        }
        OnlineSource::DesktopHut => {
            let items =
                fetch_desktophut_wallpapers(client, page, query, category_value.unwrap_or("all"))
                    .await?;
            Ok((items, true))
        }
        OnlineSource::MotionBGS => {
            let items = fetch_motionbgs_wallpapers(
                client,
                page,
                query,
                category_value.unwrap_or("all"),
                resolution_value.unwrap_or("all"),
            )
            .await?;
            Ok((items, true))
        }
        OnlineSource::LiveWallpapers4Free => {
            let result = fetch_livewallpapers4free_page(
                client,
                page,
                query,
                category_value.unwrap_or("all"),
                resolution_value.unwrap_or("4k"),
            )
            .await?;
            Ok((result.items, result.has_more))
        }
        OnlineSource::MoeWalls => {
            let items =
                fetch_moewalls_wallpapers(client, page, query, category_value.unwrap_or("all"))
                    .await?;
            Ok((items, true))
        }
        OnlineSource::WallpaperWaifu => {
            let items = fetch_wallpaperwaifu_wallpapers(
                client,
                page,
                query,
                category_value.unwrap_or("all"),
            )
            .await?;
            Ok((items, true))
        }
        OnlineSource::Wallhaven => {
            let items = fetch_wallhaven_wallpapers(
                client,
                page,
                query,
                category_value.unwrap_or("110"),
                "toplist",
                resolution_value.unwrap_or("all"),
            )
            .await?;
            Ok((items, true))
        }
        OnlineSource::Bing => {
            // Bing only offers the daily image; it is not searchable and only
            // participates on the first page of an unfiltered search.
            if page > 1 || query.is_some() {
                return Ok((Vec::new(), false));
            }
            let items = fetch_bing_wallpapers(client).await?;
            Ok((items, false))
        }
        OnlineSource::Minimalistic => {
            if page > 1 {
                return Ok((Vec::new(), false));
            }
            let mut items = fetch_minimalistic_wallpapers(client).await?;
            if let Some(query) = query {
                let needle = query.to_lowercase();
                items.retain(|item| {
                    item.title.to_lowercase().contains(&needle)
                        || item.author_or_copyright.to_lowercase().contains(&needle)
                });
            }
            if items.is_empty() {
                return Err("no matches".into());
            }
            items.truncate(24);
            Ok((items, false))
        }
        OnlineSource::Unified => Err("invalid source".into()),
    }
}

/// Turn a catalogue item into a streamable media URL.
///
/// DesktopHut, MoeWalls and WallpaperWaifu hide the media behind a detail
/// page; the rest of the catalogues already store the media URL.
pub async fn resolve_download_url(
    client: &reqwest::Client,
    item: &OnlineWallpaperItem,
) -> Result<String, String> {
    if !item.source.needs_resolution() {
        return Ok(item.full_url.clone());
    }

    let source = item.source.label();
    let html = client
        .get(&item.full_url)
        .timeout(std::time::Duration::from_secs(20))
        .header(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0",
        )
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .send()
        .await
        .map_err(|e| format!("Could not open the {} page: {}", source, e))?
        .error_for_status()
        .map_err(|e| format!("{} returned an HTTP error: {}", source, e))?
        .text()
        .await
        .map_err(|e| format!("Could not read the {} page: {}", source, e))?;

    let resolved = match item.source {
        OnlineSource::DesktopHut => parse_desktophut_download(&html),
        OnlineSource::MoeWalls => parse_moewalls_download(&html),
        OnlineSource::WallpaperWaifu => parse_wallpaperwaifu_download(&html),
        _ => None,
    };
    resolved.ok_or_else(|| {
        format!(
            "{} did not expose a download link for this wallpaper",
            source
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unified_never_contains_pseudo_source() {
        assert!(!OnlineSource::catalogs().contains(&OnlineSource::Unified));
        assert_eq!(OnlineSource::catalogs().len(), 9);
    }

    #[tokio::test]
    #[ignore]
    async fn live_download_first_item_probe() {
        const BROWSER: &str =
            "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0";

        let client = reqwest::Client::new();

        async fn check(client: &reqwest::Client, label: &str, url: &str) {
            let response = client
                .get(url)
                .header(reqwest::header::USER_AGENT, BROWSER)
                .header(reqwest::header::RANGE, "bytes=0-1023")
                .send()
                .await;
            match response {
                Ok(response) => eprintln!(
                    "{label}: status={} final={} content-type={:?}",
                    response.status(),
                    response.url(),
                    response
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok())
                ),
                Err(error) => eprintln!("{label}: request error: {error}"),
            }
        }

        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();

        let cases: Vec<(OnlineSource, OnlineWallpaperItem)> = {
            let mut cases = Vec::new();
            if let Ok(items) = fetch_moewalls_wallpapers(&client, 1, None, "all").await {
                if let Some(item) = items.first() {
                    cases.push((OnlineSource::MoeWalls, item.clone()));
                }
            }
            if let Ok(items) = fetch_wallpaperwaifu_wallpapers(&client, 1, None, "all").await {
                if let Some(item) = items.first() {
                    cases.push((OnlineSource::WallpaperWaifu, item.clone()));
                }
            }
            if let Ok(items) = fetch_desktophut_wallpapers(&client, 1, None, "all").await {
                if let Some(item) = items.first() {
                    cases.push((OnlineSource::DesktopHut, item.clone()));
                }
            }
            cases
        };

        for (source, item) in cases {
            eprintln!("\n=== {} {} ===", source.label(), item.title);
            eprintln!("page: {}", item.full_url);
            match resolve_download_url(&client, &item).await {
                Ok(url) => {
                    eprintln!("resolved: {url}");
                    check(&client, "resolved RANGE", &url).await;
                }
                Err(error) => eprintln!("resolve error: {error}"),
            }

            let bust = format!(
                "{}{}aura={}",
                item.full_url,
                if item.full_url.contains('?') { '&' } else { '?' },
                stamp
            );
            let html = client
                .get(&bust)
                .header(reqwest::header::USER_AGENT, BROWSER)
                .header(reqwest::header::CACHE_CONTROL, "no-cache")
                .send()
                .await;
            let html = match html {
                Ok(response) => response.text().await.ok(),
                Err(_) => None,
            };
            if let Some(html) = html {
                let parsed = match source {
                    OnlineSource::MoeWalls => parse_moewalls_download(&html),
                    OnlineSource::WallpaperWaifu => parse_wallpaperwaifu_download(&html),
                    OnlineSource::DesktopHut => parse_desktophut_download(&html),
                    _ => None,
                };
                match parsed {
                    Some(url) => {
                        eprintln!("busted: {url}");
                        check(&client, "busted RANGE", &url).await;
                    }
                    None => eprintln!("busted: no token in {}", bust),
                }
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn live_download_variants() {
        use reqwest::redirect::Policy;

        const PAGE: &str =
            "https://moewalls.com/anime/claude-chan-afternoon-daydream-live-wallpaper/";
        const BROWSER: &str =
            "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0";
        const AURA: &str = concat!("Aura-LiveWallpaper-Client/", env!("CARGO_PKG_VERSION"));

        let plain = reqwest::Client::new();
        let no_redirect = reqwest::Client::builder()
            .redirect(Policy::none())
            .build()
            .expect("no redirect client");
        let http1 = reqwest::Client::builder()
            .http1_only()
            .build()
            .expect("http1 client");
        let http1_no_redirect = reqwest::Client::builder()
            .http1_only()
            .redirect(Policy::none())
            .build()
            .expect("http1 no redirect client");

        async fn variant(
            name: &str,
            page_client: &reqwest::Client,
            download_client: &reqwest::Client,
            ua: &str,
        ) {
            let html = page_client
                .get(PAGE)
                .header(reqwest::header::USER_AGENT, BROWSER)
                .send()
                .await
                .expect("page")
                .text()
                .await
                .expect("html");
            let Some(url) = parse_moewalls_download(&html) else {
                eprintln!("{name}: no token in page (len {})", html.len());
                return;
            };
            let response = download_client
                .get(&url)
                .header(reqwest::header::USER_AGENT, ua)
                .send()
                .await
                .expect("download request");
            eprintln!(
                "{name}: status={} location={:?} content-type={:?}",
                response.status(),
                response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok()),
                response
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok())
            );
        }

        variant("plain+browser", &plain, &no_redirect, BROWSER).await;
        variant("plain+aura", &plain, &no_redirect, AURA).await;
        variant("http1+browser", &http1, &http1_no_redirect, BROWSER).await;
        variant("http1+aura", &http1, &http1_no_redirect, AURA).await;
    }

    #[tokio::test]
    #[ignore]
    async fn live_download_raw_probe() {
        let client = reqwest::Client::new();
        let items = fetch_moewalls_wallpapers(&client, 1, None, "all")
            .await
            .expect("moewalls");
        let url = resolve_download_url(&client, &items[0])
            .await
            .expect("resolved");
        eprintln!("url: {url}");
        let mut response = client.get(&url).send().await.expect("send");
        eprintln!("status: {}", response.status());
        eprintln!("final url: {}", response.url());
        for (name, value) in response.headers() {
            eprintln!("header {name}: {value:?}");
        }
        if let Ok(Some(chunk)) = response.chunk().await {
            let preview = String::from_utf8_lossy(&chunk[..chunk.len().min(1200)]);
            eprintln!("chunk {} bytes: {preview}", chunk.len());
        }
    }

    /// Network smoke test for the tokenised catalogues. It cancels after
    /// ~1 MB so the test stays fast while exercising the real download path.
    #[tokio::test]
    #[ignore]
    async fn live_tokenised_downloads_smoke() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let client = reqwest::Client::new();

        async fn probe(client: &reqwest::Client, source: OnlineSource) {
            let item = match source {
                OnlineSource::MoeWalls => fetch_moewalls_wallpapers(client, 1, None, "all")
                    .await
                    .map(|items| items[0].clone()),
                OnlineSource::WallpaperWaifu => {
                    fetch_wallpaperwaifu_wallpapers(client, 1, None, "all")
                        .await
                        .map(|items| items[0].clone())
                }
                OnlineSource::DesktopHut => {
                    fetch_desktophut_wallpapers(client, 1, None, "all")
                        .await
                        .map(|items| items[0].clone())
                }
                _ => unreachable!(),
            }
            .expect("catalogue item");

            let url = resolve_download_url(client, &item)
                .await
                .expect("resolved download url");
            eprintln!("{} resolved: {}", source.label(), url);

            let dest = std::env::temp_dir().join(format!(
                "aura-probe-{}.mp4",
                item.id.replace(['/', '\\', ' ', ':'], "_")
            ));
            let _ = std::fs::remove_file(&dest);
            let cancel = Arc::new(AtomicBool::new(false));
            let cancel_cb = cancel.clone();
            let result = download_to_file_with_progress(
                client,
                &url,
                &dest,
                Some(item.full_url.as_str()),
                Some(cancel),
                move |downloaded, total, percent| {
                    eprintln!(
                        "{} progress: {downloaded}/{total:?} ({:.0}%)",
                        source.label(),
                        percent * 100.0
                    );
                    if downloaded > 1024 * 1024 {
                        cancel_cb.store(true, Ordering::SeqCst);
                    }
                },
            )
            .await;
            let _ = std::fs::remove_file(&dest);
            match result {
                Ok(_) => eprintln!("{} download OK", source.label()),
                Err(error) if error.contains("canceled by user") => {
                    eprintln!("{} download reached 1 MB OK", source.label())
                }
                Err(error) => panic!("{} download failed: {error}", source.label()),
            }
        }

        probe(&client, OnlineSource::MoeWalls).await;
        probe(&client, OnlineSource::WallpaperWaifu).await;
        probe(&client, OnlineSource::DesktopHut).await;
    }

    /// Network smoke test, run manually with `--ignored`.
    #[tokio::test]
    #[ignore]
    async fn live_catalogs_smoke() {
        let client = reqwest::Client::new();

        let lively = fetch_lively_wallpapers(&client, 1, None)
            .await
            .expect("lively");
        eprintln!("lively: {} ({})", lively.len(), lively[0].title);

        let dh = fetch_desktophut_wallpapers(&client, 1, None, "all")
            .await
            .expect("desktophut");
        eprintln!("desktophut: {} ({})", dh.len(), dh[0].title);
        let resolved = resolve_download_url(&client, &dh[0])
            .await
            .expect("desktophut download");
        assert!(resolved.ends_with(".mp4"), "unexpected URL: {resolved}");
        eprintln!("resolved: {resolved}");

        let mw = fetch_moewalls_wallpapers(&client, 1, None, "all")
            .await
            .expect("moewalls");
        eprintln!("moewalls: {} ({})", mw.len(), mw[0].title);
        let moe_url = resolve_download_url(&client, &mw[0])
            .await
            .expect("moewalls download");
        assert!(moe_url.contains("download.php?video="), "{moe_url}");

        let ww = fetch_wallpaperwaifu_wallpapers(&client, 1, None, "all")
            .await
            .expect("wallpaperwaifu");
        eprintln!("wallpaperwaifu: {} ({})", ww.len(), ww[0].title);
        let ww_url = resolve_download_url(&client, &ww[0])
            .await
            .expect("wallpaperwaifu download");
        assert!(ww_url.contains("download.php?video="), "{ww_url}");

        let unified = fetch_unified_wallpapers(
            &client,
            1,
            Some("gojo"),
            None,
            Some(CategoryFilter::Anime),
            Some(ResolutionFilter::Uhd4K),
        )
        .await
        .expect("unified");
        eprintln!(
            "unified: {} items, errors {:?}, first {:?}",
            unified.items.len(),
            unified.errors,
            unified.items.first().map(|i| (i.source, i.title.clone()))
        );
        assert!(!unified.items.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn live_thumbnail_probe() {
        let client = reqwest::Client::new();
        let dir = std::env::temp_dir().join("aura-thumb-probe");
        let _ = std::fs::create_dir_all(&dir);

        for source in OnlineSource::catalogs() {
            match fetch_from_source(&client, *source, 1, None, None, None).await {
                Ok((items, _)) => {
                    for item in items.iter().take(2) {
                        let dest = dir.join(format!(
                            "{}.{}",
                            item.id.replace(|c: char| !c.is_ascii_alphanumeric(), "_"),
                            item.thumb_extension()
                        ));
                        let _ = std::fs::remove_file(&dest);
                        match download_online_thumbnail(
                            &client,
                            &item.thumb_url,
                            &item.full_url,
                            &dest,
                        )
                        .await
                        {
                            Ok(path) => eprintln!(
                                "{} OK {} -> {}",
                                source.label(),
                                item.thumb_url,
                                path.display()
                            ),
                            Err(error) => eprintln!(
                                "{} FAIL {} -> {}",
                                source.label(),
                                item.thumb_url,
                                error
                            ),
                        }
                    }
                }
                Err(error) => eprintln!("{} FETCH FAIL: {}", source.label(), error),
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn live_desktophut_page_two_probe() {
        let client = reqwest::Client::new();
        let items = fetch_desktophut_wallpapers(&client, 2, None, "all")
            .await
            .expect("page 2");
        let item = &items[0];
        eprintln!("page2 url: {}", item.full_url);
        assert!(!item.full_url.contains("?page="), "{}", item.full_url);
        let resolved = resolve_download_url(&client, item)
            .await
            .expect("resolve page 2 item");
        eprintln!("page2 resolved: {resolved}");
        let response = client
            .get(&resolved)
            .header(reqwest::header::USER_AGENT, "Aura-LiveWallpaper-Client/test")
            .header(reqwest::header::RANGE, "bytes=0-1023")
            .send()
            .await
            .expect("download page 2 item");
        eprintln!(
            "page2 status={} type={:?}",
            response.status(),
            response.headers().get(reqwest::header::CONTENT_TYPE)
        );
        assert!(
            response.status().is_success(),
            "unexpected status {}",
            response.status()
        );
    }
}
