use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_DOWNLOAD_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_THUMBNAIL_SOURCE_BYTES: usize = 50 * 1024 * 1024;
const MAX_THUMBNAIL_BYTES: u64 = 25 * 1024 * 1024;

fn unique_destination(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("wallpaper");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("bin");
    for index in 2..10_000 {
        let candidate = parent.join(format!("{} ({}).{}", stem, index, ext));
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}

fn starts_like_error_document(bytes: &[u8]) -> bool {
    let first = bytes
        .iter()
        .copied()
        .find(|b| !b.is_ascii_whitespace());
    matches!(first, Some(b'<') | Some(b'{') | Some(b'['))
}

/// Determines the user's standard Pictures / Imágenes directory via XDG or standard paths.
pub fn user_pictures_dir() -> PathBuf {
    #[cfg(test)]
    crate::test_support::init();

    // 1. Try parsing $XDG_CONFIG_HOME/user-dirs.dirs (instant in-memory file read, avoids spawning subprocesses)
    if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(&home);
        let user_dirs = crate::config::config_home().join("user-dirs.dirs");
        if let Ok(content) = std::fs::read_to_string(user_dirs) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("XDG_PICTURES_DIR=") {
                    let val = trimmed.trim_start_matches("XDG_PICTURES_DIR=").trim_matches('"');
                    let expanded = val.replace("$HOME", &home);
                    let p = PathBuf::from(expanded);
                    if p.exists() {
                        return p;
                    }
                }
            }
        }

        // 2. Fallbacks: Spanish 'Imágenes' or English 'Pictures'
        let p_esp = home_path.join("Imágenes");
        if p_esp.exists() {
            return p_esp;
        }
        let p_eng = home_path.join("Pictures");
        if p_eng.exists() {
            return p_eng;
        }
    }

    // 3. Fallback to xdg-user-dir PICTURES command if file wasn't found
    if let Ok(output) = std::process::Command::new("xdg-user-dir").arg("PICTURES").output() {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                let p = PathBuf::from(s);
                if p.is_absolute() {
                    return p;
                }
            }
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join("Pictures");
    }

    PathBuf::from("/tmp")
}

pub fn wallpapers_online_dir() -> PathBuf {
    #[cfg(test)]
    crate::test_support::init();

    static CACHED: OnceLock<PathBuf> = OnceLock::new();
    CACHED.get_or_init(|| {
        let pics = user_pictures_dir();
        let dir = pics.join("Wallpapers/Aura");
        let _ = std::fs::create_dir_all(&dir);

        // One-time automatic migration of any existing wallpapers from legacy hidden folder
        let legacy_base = if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                PathBuf::from(xdg)
            } else {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
                PathBuf::from(home).join(".local/share")
            }
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".local/share")
        };
        let legacy_dir = legacy_base.join("aura/wallpapers/online");
        if legacy_dir.exists() && legacy_dir != dir {
            if let Ok(entries) = std::fs::read_dir(&legacy_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let src = entry.path();
                    if src.is_file() {
                        let dst = dir.join(entry.file_name());
                        if !dst.exists() {
                            let _ = std::fs::copy(&src, &dst);
                        }
                        let _ = std::fs::remove_file(&src);
                    }
                }
            }
        }

        migrate_legacy_bing_wallpapers(&dir);

        dir
    }).clone()
}

pub fn thumbs_online_cache_dir() -> PathBuf {
    #[cfg(test)]
    crate::test_support::init();

    static CACHED: OnceLock<PathBuf> = OnceLock::new();
    CACHED.get_or_init(|| {
        let base = if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
            if !xdg.is_empty() {
                PathBuf::from(xdg)
            } else {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
                PathBuf::from(home).join(".cache")
            }
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".cache")
        };
        let dir = base.join("aura/online_thumbs");
        let _ = std::fs::create_dir_all(&dir);

        // One-time cleanup of legacy bloated uncompressed PNG/TMP thumbnails (>400KB)
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let p = entry.path();
                if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                    if name.starts_with("minimal_") && (name.ends_with(".png") || name.ends_with(".tmp")) {
                        let _ = std::fs::remove_file(p);
                    }
                }
            }
        }

        dir
    }).clone()
}

/// Sanitizes a wallpaper title to be safe for filenames across Linux, NTFS, and FAT filesystems
/// while preserving clean, natural, human-readable words in any language.
pub fn clean_title_filename(title: &str) -> String {
    let mut cleaned = String::with_capacity(title.len());
    for c in title.chars() {
        match c {
            '/' | '\\' | ':' | '|' => cleaned.push('-'),
            '*' | '?' | '"' | '<' | '>' => cleaned.push(' '),
            c if c.is_control() => cleaned.push(' '),
            c => cleaned.push(c),
        }
    }

    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let result = words.join(" ");
    let trimmed = result.trim_matches(|c: char| c == '.' || c == '-' || c == '_' || c == ' ');

    if trimmed.is_empty() {
        "Bing Wallpaper".to_string()
    } else if trimmed.chars().count() > 80 {
        let truncated: String = trimmed.chars().take(80).collect();
        truncated.trim_end_matches(|c: char| c == '.' || c == '-' || c == '_' || c == ' ').to_string()
    } else {
        trimmed.to_string()
    }
}

/// Automatically migrates legacy downloaded Bing wallpapers (e.g. `bing_adfdb411e...jpg`)
/// in the user's wallpapers directory to friendly human-readable titles (e.g. `Fields of gold.jpg`).
pub fn migrate_legacy_bing_wallpapers(dir: &Path) {
    if !dir.exists() {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let cache_dir = thumbs_online_cache_dir();
    let mut hash_map = std::collections::HashMap::new();

    let try_paths = [
        cache_dir.join("bing_daily.json"),
        cache_dir.join("bing_archive.json"),
    ];

    for p in &try_paths {
        if let Ok(content) = std::fs::read_to_string(p) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                let list_opt = val["images"].as_array().or_else(|| val["data"].as_array());
                if let Some(list) = list_opt {
                    for item in list {
                        let title = item["title"].as_str().unwrap_or_default().trim();
                        if title.is_empty() {
                            continue;
                        }
                        if let Some(hsh) = item["hsh"].as_str() {
                            if !hsh.is_empty() {
                                hash_map.insert(hsh.to_string(), title.to_string());
                            }
                        }
                        if let Some(urlbase) = item["urlbase"].as_str() {
                            let clean_url = urlbase.trim_start_matches("/th?id=");
                            hash_map.insert(clean_url.to_string(), title.to_string());
                        }
                    }
                }
            }
        }
    }

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = match path.file_name().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => continue,
        };

        if !file_name.starts_with("bing_") {
            continue;
        }

        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("jpg");
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        let raw_id = stem.trim_start_matches("bing_");

        let resolved_title = hash_map.get(raw_id).cloned().or_else(|| {
            if raw_id.contains("OHR") {
                let part = raw_id.split('.').nth(1).unwrap_or(raw_id);
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
                        return Some(s);
                    }
                }
            }
            None
        });

        if let Some(title) = resolved_title {
            let clean = clean_title_filename(&title);
            let target_name = format!("{}.{}", clean, ext);
            let target_path = dir.join(&target_name);
            if !target_path.exists() {
                if std::fs::rename(&path, &target_path).is_ok() {
                    let old_thumb = crate::scanner::thumbs::thumb_path_for_video(&path);
                    let new_thumb = crate::scanner::thumbs::thumb_path_for_video(&target_path);
                    if old_thumb.exists() && !new_thumb.exists() {
                        let _ = std::fs::rename(&old_thumb, &new_thumb);
                    }
                }
            } else {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

pub fn clean_filename(prefix: &str, id: &str, ext: &str) -> String {
    let clean_id: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    format!("{}_{}.{}", prefix, clean_id, ext)
}

#[allow(dead_code)]
pub fn is_downloaded(prefix: &str, id: &str) -> Option<PathBuf> {
    let dir = wallpapers_online_dir();
    let extensions = ["jpg", "jpeg", "png", "webp"];
    for ext in &extensions {
        let candidate = dir.join(clean_filename(prefix, id, ext));
        if candidate.exists() {
            if let Ok(metadata) = candidate.metadata() {
                if metadata.len() > 1024 {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

#[allow(dead_code)]
pub async fn download_to_file(client: &reqwest::Client, url: &str, destination: &Path) -> Result<PathBuf, String> {
    download_to_file_with_progress(client, url, destination, None, |_, _, _| {}).await
}

pub async fn download_to_file_with_progress<F>(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
    cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    mut progress_cb: F,
) -> Result<PathBuf, String>
where
    F: FnMut(u64, Option<u64>, f32) + Send + 'static,
{
    use std::sync::atomic::Ordering;
    use tokio::io::AsyncWriteExt;

    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("Unsupported download URL scheme".into());
    }

    if let Some(parent) = destination.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    // Check if cancelled before starting
    if let Some(ref flag) = cancel_flag {
        if flag.load(Ordering::Relaxed) {
            return Err("Download canceled by user".into());
        }
    }

    // Connect with user agent; do not set a rigid total download timeout so slow/large downloads can finish.
    let response = client
        .get(url)
        .header("User-Agent", concat!("Aura-LiveWallpaper-Client/", env!("CARGO_PKG_VERSION")))
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned HTTP error {}", response.status()));
    }

    if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
        let content_type = content_type.to_str().unwrap_or_default().to_ascii_lowercase();
        if content_type.starts_with("text/")
            || content_type.contains("json")
            || content_type.contains("html")
        {
            return Err(format!(
                "Server returned an unexpected content type: {}",
                content_type
            ));
        }
    }

    let total_len = response.content_length();
    if let Some(total) = total_len {
        if total > MAX_DOWNLOAD_BYTES {
            return Err(format!(
                "File is too large ({} bytes, maximum {})",
                total, MAX_DOWNLOAD_BYTES
            ));
        }
    }

    let destination = unique_destination(destination);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_path = destination.with_file_name(format!(
        "{}.tmp.{}.{}",
        destination
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("wallpaper"),
        std::process::id(),
        stamp
    ));
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| format!("Failed to create temporary file: {}", e))?;

    let mut response = response;
    let mut total_bytes: u64 = 0;
    let mut prefix = Vec::with_capacity(64);
    let mut last_progress_time = std::time::Instant::now();

    // Initial progress report (0%)
    progress_cb(0, total_len, 0.0);

    loop {
        // Check if download was canceled by user
        if let Some(ref flag) = cancel_flag {
            if flag.load(Ordering::Relaxed) {
                drop(file);
                let _ = tokio::fs::remove_file(&tmp_path).await;
                return Err("Download canceled by user".into());
            }
        }

        // Per-chunk idle timeout (45 seconds without any chunk received)
        let chunk_res = tokio::time::timeout(Duration::from_secs(45), response.chunk()).await;
        let chunk_opt = match chunk_res {
            Ok(Ok(opt)) => opt,
            Ok(Err(e)) => {
                drop(file);
                let _ = tokio::fs::remove_file(&tmp_path).await;
                return Err(format!("Error while downloading stream: {}", e));
            }
            Err(_) => {
                drop(file);
                let _ = tokio::fs::remove_file(&tmp_path).await;
                return Err("Download connection timed out (server stopped sending data for 45s)".into());
            }
        };

        let chunk = match chunk_opt {
            Some(c) => c,
            None => break, // Stream complete
        };

        if prefix.len() < 64 {
            let take = (64 - prefix.len()).min(chunk.len());
            prefix.extend_from_slice(&chunk[..take]);
        }

        if let Err(e) = file.write_all(&chunk).await {
            drop(file);
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(format!("Failed to write chunk to disk: {}", e));
        }

        total_bytes += chunk.len() as u64;
        if total_bytes > MAX_DOWNLOAD_BYTES {
            drop(file);
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(format!(
                "File exceeded the maximum size of {} bytes",
                MAX_DOWNLOAD_BYTES
            ));
        }

        // Throttle updates to at most once every 100ms
        if last_progress_time.elapsed() >= Duration::from_millis(100) {
            let pct = if let Some(tot) = total_len {
                if tot > 0 { (total_bytes as f32 / tot as f32).clamp(0.0, 1.0) } else { 0.0 }
            } else {
                0.0
            };
            progress_cb(total_bytes, total_len, pct);
            last_progress_time = std::time::Instant::now();
        }
    }

    if total_bytes == 0 {
        drop(file);
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err("Received empty file from server".into());
    }

    if starts_like_error_document(&prefix) {
        drop(file);
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err("Server returned an HTML/JSON document instead of media".into());
    }

    let is_image_destination = destination
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| crate::scanner::IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false);
    if is_image_destination && !is_image_data(&prefix) {
        drop(file);
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err("Downloaded file is not a valid image".into());
    }

    // Final progress report (100%)
    if let Some(tot) = total_len {
        progress_cb(total_bytes, Some(tot), 1.0);
    } else {
        progress_cb(total_bytes, None, 1.0);
    }

    if let Err(e) = file.flush().await {
        drop(file);
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(format!("Failed to flush downloaded file: {}", e));
    }
    if let Err(e) = file.sync_all().await {
        drop(file);
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(format!("Failed to sync downloaded file: {}", e));
    }
    drop(file);

    if let Err(e) = tokio::fs::rename(&tmp_path, &destination).await {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(format!("Failed to finalize downloaded file: {}", e));
    }

    Ok(destination)
}

/// Verifies whether raw bytes belong to a valid image format (JPEG, PNG, WEBP, GIF).
pub fn is_image_data(bytes: &[u8]) -> bool {
    if bytes.len() < 12 {
        return false;
    }
    // JPEG magic bytes: FF D8 FF
    if bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return true;
    }
    // PNG magic bytes: \x89 P N G
    if &bytes[0..4] == b"\x89PNG" {
        return true;
    }
    // WEBP magic bytes: RIFF .... WEBP
    if &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return true;
    }
    // GIF magic bytes: GIF
    if &bytes[0..3] == b"GIF" {
        return true;
    }
    false
}

/// Decodes an image from memory, downscales it to fit within max dimensions, and saves as JPEG.
pub fn save_downscaled_image(bytes: &[u8], destination: &Path, max_w: u32, max_h: u32) -> Result<PathBuf, String> {
    if bytes.len() > MAX_THUMBNAIL_SOURCE_BYTES {
        return Err("Image is too large to process safely".into());
    }
    if let Some(parent) = destination.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("Failed to decode image: {}", e))?;
    let thumb = img.thumbnail(max_w, max_h);
    let tmp_path = destination.with_extension("tmp.jpg");
    if let Err(e) = thumb.save(&tmp_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("Failed to encode/save thumbnail: {}", e));
    }
    if let Err(e) = std::fs::rename(&tmp_path, destination) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("Failed to finalize thumbnail: {}", e));
    }
    Ok(destination.to_path_buf())
}

/// Downscales an existing on-disk image file in place to reduce storage and RAM footprint.
pub fn downscale_image_file_in_place(file_path: &Path, max_w: u32, max_h: u32) -> Result<PathBuf, String> {
    if let Ok(meta) = file_path.metadata() {
        if meta.len() > MAX_THUMBNAIL_SOURCE_BYTES as u64 {
            return Err("Image is too large to process safely".into());
        }
    }
    let img = image::open(file_path)
        .map_err(|e| format!("Failed to open image {}: {}", file_path.display(), e))?;
    let thumb = img.thumbnail(max_w, max_h);
    let tmp_path = file_path.with_extension("tmp.jpg");
    if let Err(e) = thumb.save(&tmp_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("Failed to encode/save downscaled thumbnail: {}", e));
    }
    if let Err(e) = std::fs::rename(&tmp_path, file_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("Failed to finalize downscaled image: {}", e));
    }
    Ok(file_path.to_path_buf())
}

/// Downloads and caches an online wallpaper thumbnail with concurrency control and multi-tier fallback.
/// Tier 1: Fast CDN thumbnail (e.g. wsrv.nl or provider thumbnail).
/// Tier 2: Direct source download with local downscaling.
pub async fn download_online_thumbnail(
    client: &reqwest::Client,
    thumb_url: &str,
    fallback_full_url: &str,
    destination: &Path,
) -> Result<PathBuf, String> {
    static THUMB_SEMAPHORE: OnceLock<tokio::sync::Semaphore> = OnceLock::new();
    let sem = THUMB_SEMAPHORE.get_or_init(|| tokio::sync::Semaphore::new(4));
    let _permit = sem.acquire().await.map_err(|e| e.to_string())?;

    if let Some(parent) = destination.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Reuse existing compact thumbnail if valid (<400KB and >500B)
    if let Ok(meta) = destination.metadata() {
        if meta.is_file() && meta.len() > 500 && meta.len() <= 400_000 {
            return Ok(destination.to_path_buf());
        }
        // Downscale oversized legacy file in place
        if meta.is_file() && meta.len() > 400_000 {
            let dest_clone = destination.to_path_buf();
            let opt_res = tokio::task::spawn_blocking(move || {
                downscale_image_file_in_place(&dest_clone, 400, 225)
            }).await;
            if let Ok(Ok(path)) = opt_res {
                return Ok(path);
            }
        }
    }

    // Step 1: Fast CDN Web Thumbnail (timeout 6s)
    let req_res = client
        .get(thumb_url)
        .timeout(Duration::from_secs(6))
        .header("User-Agent", concat!("Aura-LiveWallpaper-Client/", env!("CARGO_PKG_VERSION")))
        .send()
        .await;

    if let Ok(resp) = req_res {
        if resp.status().is_success()
            && resp
                .content_length()
                .map(|len| len <= MAX_THUMBNAIL_BYTES)
                .unwrap_or(true)
        {
            if let Ok(bytes) = resp.bytes().await {
                if bytes.len() <= MAX_THUMBNAIL_SOURCE_BYTES && is_image_data(&bytes) {
                    if bytes.len() <= 400_000 {
                        let tmp_path = destination.with_extension("tmp.jpg");
                        if tokio::fs::write(&tmp_path, &bytes).await.is_ok() {
                            if tokio::fs::rename(&tmp_path, destination).await.is_ok() {
                                return Ok(destination.to_path_buf());
                            }
                        }
                    } else {
                        let dest_clone = destination.to_path_buf();
                        let opt_res = tokio::task::spawn_blocking(move || {
                            save_downscaled_image(&bytes, &dest_clone, 400, 225)
                        }).await;
                        if let Ok(Ok(path)) = opt_res {
                            return Ok(path);
                        }
                    }
                }
            }
        }
    }

    // Step 2: Fallback to full_url and downscale locally
    if !fallback_full_url.is_empty() && fallback_full_url != thumb_url {
        let full_resp = client
            .get(fallback_full_url)
            .timeout(Duration::from_secs(20))
            .header("User-Agent", concat!("Aura-LiveWallpaper-Client/", env!("CARGO_PKG_VERSION")))
            .send()
            .await
            .map_err(|e| format!("Fallback download failed: {}", e))?;

        if full_resp.status().is_success() {
            if full_resp
                .content_length()
                .map(|len| len > MAX_THUMBNAIL_BYTES)
                .unwrap_or(false)
            {
                return Err("Fallback thumbnail is too large".into());
            }
            let bytes = full_resp
                .bytes()
                .await
                .map_err(|e| format!("Failed to read fallback bytes: {}", e))?;

            if bytes.len() <= MAX_THUMBNAIL_SOURCE_BYTES && is_image_data(&bytes) {
                let dest_clone = destination.to_path_buf();
                let opt_res = tokio::task::spawn_blocking(move || {
                    save_downscaled_image(&bytes, &dest_clone, 400, 225)
                }).await;
                if let Ok(Ok(path)) = opt_res {
                    return Ok(path);
                }
            }
        }
    }

    Err(format!("Could not acquire valid thumbnail for {}", destination.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_filename() {
        let name = clean_filename("bing", "OHR.MountFuji_EN-US123", "jpg");
        assert_eq!(name, "bing_OHR_MountFuji_EN-US123.jpg");

        let name_symbols = clean_filename("wallhaven", "x9k!@#$%-test", "png");
        assert_eq!(name_symbols, "wallhaven_x9k_____-test.png");
    }

    #[test]
    fn test_clean_title_filename() {
        assert_eq!(clean_title_filename("Fields of gold"), "Fields of gold");
        assert_eq!(clean_title_filename("Mount Fuji: Japan / Sunset"), "Mount Fuji- Japan - Sunset");
        assert_eq!(clean_title_filename("  The  \"Great\"  Wave  "), "The Great Wave");
        assert_eq!(clean_title_filename("???"), "Bing Wallpaper");
        assert_eq!(clean_title_filename(""), "Bing Wallpaper");
    }

    #[test]
    fn test_directories_non_empty() {
        let wall_dir = wallpapers_online_dir();
        let thumb_dir = thumbs_online_cache_dir();
        assert!(wall_dir.to_string_lossy().contains("Wallpapers/Aura") || wall_dir.to_string_lossy().contains("aura"));
        assert!(thumb_dir.to_string_lossy().contains("online_thumbs"));
    }

    #[test]
    fn test_is_image_data() {
        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(is_image_data(&jpeg));

        let png = [0x89, b'P', b'N', b'G', 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(is_image_data(&png));

        let webp = [b'R', b'I', b'F', b'F', 0, 0, 0, 0, b'W', b'E', b'B', b'P'];
        assert!(is_image_data(&webp));

        let invalid = b"<html>404 Not Found</html>";
        assert!(!is_image_data(invalid));
    }

    #[test]
    fn test_unique_destination_avoids_overwrite() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("wallpaper.jpg");
        assert_eq!(unique_destination(&first), first);

        std::fs::write(&first, b"data").unwrap();
        let second = unique_destination(&first);
        assert_eq!(second, temp.path().join("wallpaper (2).jpg"));

        std::fs::write(&second, b"data").unwrap();
        assert_eq!(
            unique_destination(&first),
            temp.path().join("wallpaper (3).jpg")
        );
    }

    #[test]
    fn test_error_document_detection() {
        assert!(starts_like_error_document(b"  \n<html>"));
        assert!(starts_like_error_document(b"{\"error\":\"nope\"}"));
        assert!(starts_like_error_document(b"[1,2,3]"));
        assert!(!starts_like_error_document(b"\xFF\xD8\xFF"));
        assert!(!starts_like_error_document(b""));
    }

    #[test]
    fn test_progress_calculation() {
        let total: Option<u64> = Some(10_000_000);
        let downloaded: u64 = 4_500_000;
        let pct = if let Some(tot) = total {
            (downloaded as f32 / tot as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        assert!((pct - 0.45).abs() < 0.001);
    }

    #[test]
    fn test_download_cancellation_pre_check() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            use std::sync::atomic::AtomicBool;
            use std::sync::Arc;
            let client = reqwest::Client::new();
            let cancel = Arc::new(AtomicBool::new(true));
            let dest = std::env::temp_dir().join("test_cancel_dl.mp4");
            let res = download_to_file_with_progress(&client, "http://127.0.0.1:9/dummy", &dest, Some(cancel), |_, _, _| {}).await;
            assert!(res.is_err());
            assert_eq!(res.unwrap_err(), "Download canceled by user");
            assert!(!dest.with_extension("tmp").exists());
        });
    }
}

