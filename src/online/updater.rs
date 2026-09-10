use std::path::{Path, PathBuf};
use std::process::Command;

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
        parts.get(0).copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    )
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    parse_semver(latest) > parse_semver(current)
}

pub async fn check_latest_release(client: &reqwest::Client) -> Result<(String, String, Option<String>), String> {
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

    let mut download_url = None;
    if let Some(assets) = json["assets"].as_array() {
        for asset in assets {
            let name = asset["name"].as_str().unwrap_or_default();
            if name.ends_with(".tar.gz") && name.contains("linux") {
                download_url = asset["browser_download_url"].as_str().map(|s| s.to_string());
                break;
            }
        }
    }

    Ok((latest_tag, body, download_url))
}

pub async fn perform_update(client: &reqwest::Client, download_url: &str) -> Result<String, String> {
    if is_flatpak() {
        return Err("Updates under Flatpak must be installed through your software center (COSMIC Store or Flathub).".into());
    }

    let pkg_resp = client
        .get(download_url)
        .header("User-Agent", "Aura-Updater")
        .send()
        .await
        .map_err(|e| format!("Failed to download update: {}", e))?;

    if !pkg_resp.status().is_success() {
        return Err(format!("Server returned HTTP {} when downloading package", pkg_resp.status()));
    }

    let bytes = pkg_resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read package data: {}", e))?;

    let tmp_tar = std::env::temp_dir().join("aura_update.tar.gz");
    let tmp_extract = std::env::temp_dir().join("aura_update_extracted");
    let _ = tokio::fs::create_dir_all(&tmp_extract).await;
    tokio::fs::write(&tmp_tar, &bytes)
        .await
        .map_err(|e| format!("Failed to write temporary tarball: {}", e))?;

    let tar_status = Command::new("tar")
        .args(["-xzf", &tmp_tar.to_string_lossy(), "-C", &tmp_extract.to_string_lossy()])
        .status()
        .map_err(|e| format!("Failed to execute tar: {}", e))?;

    if !tar_status.success() {
        return Err("Extraction failed".into());
    }

    let mut new_bin = tmp_extract.join("aura");
    if !new_bin.exists() {
        if let Ok(entries) = std::fs::read_dir(&tmp_extract) {
            for entry in entries.filter_map(|e| e.ok()) {
                let candidate = entry.path().join("aura");
                if candidate.exists() {
                    new_bin = candidate;
                    break;
                }
            }
        }
    }

    if !new_bin.exists() {
        return Err("Precompiled aura binary not found in extracted archive".into());
    }

    let home = std::env::var("HOME").map_err(|_| "HOME directory not set".to_string())?;
    let target_bin = PathBuf::from(&home).join(".local/bin/aura");
    if let Some(parent) = target_bin.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp_target = target_bin.with_extension("new");
    std::fs::copy(&new_bin, &tmp_target)
        .map_err(|e| format!("Failed to copy binary: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp_target, std::fs::Permissions::from_mode(0o755));
    }

    std::fs::rename(&tmp_target, &target_bin)
        .map_err(|e| format!("Failed to replace binary: {}", e))?;

    let _ = tokio::fs::remove_file(&tmp_tar).await;
    let _ = tokio::fs::remove_dir_all(&tmp_extract).await;

    Ok(target_bin.to_string_lossy().to_string())
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
}
