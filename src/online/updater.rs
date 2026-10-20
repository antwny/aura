use sha2::{Digest, Sha256};
use std::ffi::CString;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use tokio::io::AsyncWriteExt;

const MAX_UPDATE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_CHECKSUM_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub tag: String,
    pub body: String,
    pub download_url: Option<String>,
    pub checksum_url: Option<String>,
}

pub fn is_flatpak() -> bool {
    Path::new("/.flatpak-info").exists() || std::env::var_os("FLATPAK_ID").is_some()
}

pub fn parse_semver(v: &str) -> (u32, u32, u32) {
    let clean = v.trim_start_matches('v').trim();
    let parts: Vec<u32> = clean
        .split('.')
        .filter_map(|p| p.parse::<u32>().ok())
        .collect();
    (
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    )
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    parse_semver(latest) > parse_semver(current)
}

pub async fn check_latest_release(client: &reqwest::Client) -> Result<ReleaseInfo, String> {
    let resp = client
        .get("https://api.github.com/repos/antwny/aura/releases/latest")
        .header("User-Agent", "Aura-Updater")
        .send()
        .await
        .map_err(|e| format!("Error connecting to GitHub: {}", e))?;

    if !resp.status().is_success() {
        if resp.status().as_u16() == 404 {
            return Err("No public releases found on GitHub.".into());
        }
        return Err(format!("GitHub API returned status code {}", resp.status()));
    }

    let json = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse GitHub response: {}", e))?;

    let latest_tag = json["tag_name"]
        .as_str()
        .ok_or_else(|| "Missing tag_name in release".to_string())?
        .to_string();

    let body = json["body"].as_str().unwrap_or("").to_string();

    let mut package_asset: Option<(String, String)> = None;
    let mut checksum_assets: Vec<(String, String)> = Vec::new();

    if let Some(assets) = json["assets"].as_array() {
        for asset in assets {
            let Some(name) = asset["name"].as_str() else {
                continue;
            };
            let Some(url) = asset["browser_download_url"].as_str() else {
                continue;
            };
            if name.ends_with(".tar.gz") && name.contains("linux") {
                package_asset = Some((name.to_string(), url.to_string()));
            } else if name.ends_with(".sha256") && name.contains("linux") {
                checksum_assets.push((name.to_string(), url.to_string()));
            }
        }
    }

    let (download_url, checksum_url) = match package_asset {
        Some((package_name, package_url)) => {
            let expected = format!("{}.sha256", package_name);
            match checksum_assets.into_iter().find(|(name, _)| name == &expected) {
                Some((_, checksum_url)) => (Some(package_url), Some(checksum_url)),
                None => (None, None),
            }
        }
        None => (None, None),
    };

    Ok(ReleaseInfo {
        tag: latest_tag,
        body,
        download_url,
        checksum_url,
    })
}

pub async fn perform_update(
    client: &reqwest::Client,
    download_url: &str,
    checksum_url: &str,
) -> Result<String, String> {
    if is_flatpak() {
        return Err(
            "Updates under Flatpak must be installed through your software center (COSMIC Store or Flathub)."
                .into(),
        );
    }

    validate_release_url(download_url)?;
    validate_release_url(checksum_url)?;

    let checksum_text = download_text(client, checksum_url, MAX_CHECKSUM_BYTES).await?;
    let expected_hash = parse_checksum(&checksum_text)
        .ok_or_else(|| "The release checksum file is empty or invalid.".to_string())?;

    let temp_dir = tempfile::Builder::new()
        .prefix("aura-update-")
        .tempdir()
        .map_err(|e| format!("Failed to create a temporary update directory: {}", e))?;
    let archive_path = temp_dir.path().join("aura-update.tar.gz");

    let (archive_len, actual_hash) =
        download_file_with_sha256(client, download_url, &archive_path).await?;
    if archive_len == 0 {
        return Err("The downloaded update is empty.".into());
    }
    if actual_hash != expected_hash {
        return Err(format!(
            "Update checksum mismatch (expected {}, got {}).",
            expected_hash, actual_hash
        ));
    }

    validate_archive_entries(&archive_path)?;
    let extract_dir = temp_dir.path().join("extracted");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Failed to create extraction directory: {}", e))?;
    extract_archive(&archive_path, &extract_dir)?;

    let new_bin = find_archive_member(&extract_dir, "aura")
        .ok_or_else(|| "Precompiled aura binary not found in extracted archive".to_string())?;
    validate_linux_binary(&new_bin)?;

    let target_dir = install_dir()?;
    let target_bin = target_dir.join("aura");
    install_binary(&new_bin, &target_bin)?;

    if let Some(new_mpvpaper) = find_archive_member(&extract_dir, "mpvpaper") {
        validate_linux_binary(&new_mpvpaper)?;
        let target_mpvpaper = target_dir.join("mpvpaper");
        if let Err(e) = install_binary(&new_mpvpaper, &target_mpvpaper) {
            eprintln!("[Aura Updater] Aura was updated, but mpvpaper could not be replaced: {}", e);
        }
    }

    Ok(target_bin.to_string_lossy().to_string())
}

pub fn parse_checksum(text: &str) -> Option<String> {
    let hash = text.split_whitespace().next()?.trim();
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(hash.to_ascii_lowercase())
}

fn validate_release_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid release URL: {}", e))?;
    let host = parsed.host_str().unwrap_or_default();
    let loopback = host == "127.0.0.1" || host == "localhost" || host == "::1";

    let scheme_ok = parsed.scheme() == "https"
        || (cfg!(test) && loopback && parsed.scheme() == "http");
    if !scheme_ok {
        return Err("Refusing to download an update over an insecure connection.".into());
    }

    let host_ok = host == "github.com"
        || host.ends_with(".githubusercontent.com")
        || (cfg!(test) && loopback);
    if !host_ok {
        return Err(format!("Refusing to download an update from untrusted host '{}'.", host));
    }

    Ok(())
}

async fn download_text(
    client: &reqwest::Client,
    url: &str,
    max_bytes: usize,
) -> Result<String, String> {
    let response = client
        .get(url)
        .header("User-Agent", "Aura-Updater")
        .send()
        .await
        .map_err(|e| format!("Failed to download checksum: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Server returned HTTP {} when downloading checksum",
            response.status()
        ));
    }

    if let Some(length) = response.content_length() {
        if length > max_bytes as u64 {
            return Err("The release checksum file is unexpectedly large.".into());
        }
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read checksum: {}", e))?;
    if bytes.len() > max_bytes {
        return Err("The release checksum file is unexpectedly large.".into());
    }
    String::from_utf8(bytes.to_vec())
        .map_err(|_| "The release checksum file is not valid UTF-8.".to_string())
}

async fn download_file_with_sha256(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
) -> Result<(u64, String), String> {
    let response = client
        .get(url)
        .header("User-Agent", "Aura-Updater")
        .send()
        .await
        .map_err(|e| format!("Failed to download update: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Server returned HTTP {} when downloading package",
            response.status()
        ));
    }

    let expected_len = response.content_length();
    if let Some(length) = expected_len {
        if length > MAX_UPDATE_BYTES {
            return Err(format!(
                "The update package is too large ({} bytes, limit {} bytes).",
                length, MAX_UPDATE_BYTES
            ));
        }
    }

    let mut response = response;
    let mut file = tokio::fs::File::create(destination)
        .await
        .map_err(|e| format!("Failed to create temporary package: {}", e))?;
    let mut hasher = Sha256::new();
    let mut total_bytes = 0u64;

    loop {
        let chunk = tokio::time::timeout(std::time::Duration::from_secs(45), response.chunk())
            .await
            .map_err(|_| "Download connection timed out (no data for 45s).".to_string())?
            .map_err(|e| format!("Error while downloading update: {}", e))?;

        let Some(chunk) = chunk else {
            break;
        };

        total_bytes = total_bytes
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| "The downloaded update size overflowed.".to_string())?;
        if total_bytes > MAX_UPDATE_BYTES {
            return Err(format!(
                "The update package is too large (limit {} bytes).",
                MAX_UPDATE_BYTES
            ));
        }

        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write update package: {}", e))?;
    }

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush update package: {}", e))?;
    file.sync_all()
        .await
        .map_err(|e| format!("Failed to sync update package: {}", e))?;

    if let Some(expected) = expected_len {
        if total_bytes != expected {
            return Err(format!(
                "Downloaded update is incomplete (expected {} bytes, got {}).",
                expected, total_bytes
            ));
        }
    }

    Ok((total_bytes, format!("{:x}", hasher.finalize())))
}

fn validate_archive_entries(archive_path: &Path) -> Result<(), String> {
    let output = Command::new("tar")
        .args(["-tzf"])
        .arg(archive_path)
        .output()
        .map_err(|e| format!("Failed to inspect update archive: {}", e))?;

    if !output.status.success() {
        return Err("The downloaded update archive could not be read.".into());
    }

    let listing = String::from_utf8_lossy(&output.stdout);
    for line in listing.lines() {
        let entry = line.trim();
        if entry.is_empty() {
            continue;
        }
        let path = Path::new(entry);
        if path.is_absolute() {
            return Err(format!("Update archive contains an absolute path: {}", entry));
        }
        if path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(format!("Update archive contains an unsafe path: {}", entry));
        }
    }

    let verbose = Command::new("tar")
        .args(["-tvzf"])
        .arg(archive_path)
        .output()
        .map_err(|e| format!("Failed to inspect update archive links: {}", e))?;
    if !verbose.status.success() {
        return Err("The downloaded update archive could not be inspected.".into());
    }
    for line in String::from_utf8_lossy(&verbose.stdout).lines() {
        let entry_type = line.chars().next();
        if matches!(entry_type, Some('l') | Some('h')) {
            return Err("Update archive contains symbolic or hard links, which are not allowed.".into());
        }
    }

    Ok(())
}

fn extract_archive(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let status = Command::new("tar")
        .args(["-xzf"])
        .arg(archive_path)
        .arg("-C")
        .arg(destination)
        .args([
            "--no-same-owner",
            "--no-same-permissions",
            "--no-overwrite-dir",
        ])
        .status()
        .map_err(|e| format!("Failed to execute tar: {}", e))?;

    if !status.success() {
        return Err("Extraction failed".into());
    }
    Ok(())
}

fn find_archive_member(root: &Path, file_name: &str) -> Option<PathBuf> {
    walkdir::WalkDir::new(root)
        .max_depth(3)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .find(|entry| {
            entry.file_type().is_file()
                && entry.file_name().to_string_lossy() == file_name
        })
        .map(|entry| entry.into_path())
}

fn validate_linux_binary(path: &Path) -> Result<(), String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open extracted binary: {}", e))?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|_| "The extracted binary is empty or unreadable.".to_string())?;
    if &magic != b"\x7fELF" {
        return Err("The extracted file is not a valid Linux executable.".into());
    }
    Ok(())
}

fn install_dir() -> Result<PathBuf, String> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if can_write_dir(parent) {
                return Ok(parent.to_path_buf());
            }
        }
    }

    let home = std::env::var("HOME").map_err(|_| "HOME directory not set".to_string())?;
    let local_bin = PathBuf::from(home).join(".local/bin");
    std::fs::create_dir_all(&local_bin)
        .map_err(|e| format!("Cannot create ~/.local/bin: {}", e))?;
    if can_write_dir(&local_bin) {
        Ok(local_bin)
    } else {
        Err("Aura is installed in a read-only location. Reinstall it from the release archive or installer.".into())
    }
}

fn can_write_dir(path: &Path) -> bool {
    let Ok(path) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    unsafe { libc::access(path.as_ptr(), libc::W_OK) == 0 }
}

fn install_binary(source: &Path, target: &Path) -> Result<(), String> {
    let parent = target
        .parent()
        .ok_or_else(|| "Update target has no parent directory".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("Failed to create update directory: {}", e))?;

    if target.exists() {
        let backup = target.with_extension("bak");
        std::fs::copy(target, &backup)
            .map_err(|e| format!("Failed to back up {}: {}", target.display(), e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&backup, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("Failed to set backup permissions: {}", e))?;
        }
    }

    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Invalid update target filename".to_string())?;
    let temporary = parent.join(format!(".{}.new.{}", file_name, std::process::id()));

    std::fs::copy(source, &temporary)
        .map_err(|e| format!("Failed to copy binary: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to set binary permissions: {}", e))?;
    }

    if let Ok(file) = std::fs::File::open(&temporary) {
        let _ = file.sync_all();
    }

    if let Err(e) = std::fs::rename(&temporary, target) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("Failed to replace binary: {}", e));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_semver() {
        assert_eq!(parse_semver("1.1.0"), (1, 1, 0));
        assert_eq!(parse_semver("v1.1.2"), (1, 1, 2));
        assert_eq!(parse_semver("2.0.0-beta"), (2, 0, 0));
    }

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("1.1.2", "1.1.1"));
        assert!(is_newer_version("v1.2.0", "v1.1.9"));
        assert!(!is_newer_version("1.1.0", "1.1.1"));
        assert!(!is_newer_version("1.1.1", "1.1.1"));
    }

    #[test]
    fn test_parse_checksum() {
        let hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert_eq!(parse_checksum(hash), Some(hash.to_string()));
        assert_eq!(parse_checksum(&format!("{}  aura.tar.gz\n", hash)), Some(hash.to_string()));
        assert_eq!(parse_checksum("not-a-checksum"), None);
        assert_eq!(parse_checksum(""), None);
    }

    #[test]
    fn test_archive_path_validation() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("unsafe.tar.gz");
        let payload = temp.path().join("payload");
        std::fs::create_dir_all(&payload).unwrap();
        std::fs::write(payload.join("file"), b"data").unwrap();

        let status = Command::new("tar")
            .args(["-czf"])
            .arg(&archive)
            .args(["-C"])
            .arg(&payload)
            .args(["--transform", "s|file|../escape|"])
            .arg("file")
            .status()
            .unwrap();
        assert!(status.success());

        assert!(validate_archive_entries(&archive).is_err());
    }

    #[test]
    fn test_archive_link_validation() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("links.tar.gz");
        let payload = temp.path().join("payload");
        std::fs::create_dir_all(&payload).unwrap();
        std::fs::write(payload.join("file"), b"data").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("file", payload.join("link")).unwrap();

        let status = Command::new("tar")
            .args(["-czf"])
            .arg(&archive)
            .args(["-C"])
            .arg(&payload)
            .args(["file", "link"])
            .status()
            .unwrap();
        assert!(status.success());

        assert!(validate_archive_entries(&archive).is_err());
    }

    #[test]
    fn test_valid_binary_and_backup() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("aura");
        std::fs::write(&source, b"\x7fELFnew-binary").unwrap();
        std::fs::write(&target, b"old-binary").unwrap();

        install_binary(&source, &target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"\x7fELFnew-binary");
        assert_eq!(std::fs::read(target.with_extension("bak")).unwrap(), b"old-binary");
    }
}
