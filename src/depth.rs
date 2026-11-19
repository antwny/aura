//! Local depth-map generation for mouse-following wallpaper parallax.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const MODEL_URL: &str = "https://huggingface.co/onnx-community/depth-anything-v2-small/resolve/main/onnx/model_quantized.onnx";
const MODEL_NAME: &str = "depth-anything-v2-small-quantized.onnx";
const HELPER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("aura")
        .join("depth")
}

/// Location of the ONNX depth model inside Aura's cache.
pub fn model_path() -> PathBuf {
    cache_dir().join("models").join(MODEL_NAME)
}

/// Whether the depth model has been downloaded and looks complete.
pub fn model_present() -> bool {
    model_path()
        .metadata()
        .map(|metadata| metadata.is_file() && metadata.len() > 1024 * 1024)
        .unwrap_or(false)
}

pub fn model_size() -> Option<u64> {
    model_path()
        .metadata()
        .ok()
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
}

/// Checks the CPU flags exposed by the Linux kernel. The AVX flag is only
/// exposed when both the processor and OS support saving AVX state.
pub fn cpu_supports_avx() -> bool {
    std::fs::read_to_string("/proc/cpuinfo")
        .map(|cpuinfo| {
            cpuinfo.lines().any(|line| {
                let Some((field, values)) = line.split_once(':') else {
                    return false;
                };
                matches!(field.trim(), "flags" | "Features")
                    && values.split_whitespace().any(|flag| flag == "avx")
            })
        })
        .unwrap_or(false)
}

fn helper_path() -> PathBuf {
    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            let sibling = parent.join("aura-depth");
            if sibling.is_file() {
                return sibling;
            }
        }
    }
    PathBuf::from("aura-depth")
}

/// Removes the model and every generated depth map to reclaim disk space.
pub fn remove_model() -> Result<(), String> {
    let model = model_path();
    if model.is_file() {
        std::fs::remove_file(&model).map_err(|error| error.to_string())?;
    }
    let maps = cache_dir().join("maps");
    if maps.is_dir() {
        std::fs::remove_dir_all(&maps).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn source_key(image: &Path) -> String {
    let metadata = std::fs::metadata(image).ok();
    let modified = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let length = metadata.map(|metadata| metadata.len()).unwrap_or(0);
    let mut hash = Sha256::new();
    hash.update(image.to_string_lossy().as_bytes());
    hash.update(length.to_le_bytes());
    hash.update(modified.to_le_bytes());
    format!("{:x}", hash.finalize())
}

/// Returns the cached depth map path for a local image, whether or not it has
/// been generated yet.
pub fn depth_map_path(image: &Path) -> PathBuf {
    cache_dir()
        .join("maps")
        .join(format!("{}.png", source_key(image)))
}

/// Creates a normalized grayscale depth map and returns its cache path.
pub async fn generate_depth_map(image: PathBuf) -> Result<PathBuf, String> {
    let output = depth_map_path(&image);
    if output.is_file() {
        return Ok(output);
    }

    let model = model_path();
    if !model.is_file() {
        return Err("El modelo de profundidad no está descargado".into());
    }

    if !cpu_supports_avx() {
        return Err(
            "Depth maps require a CPU with AVX support; parallax is unavailable on this processor."
                .into(),
        );
    }

    let helper = helper_path();
    let mut command = tokio::process::Command::new(&helper);
    command
        .arg("--image")
        .arg(&image)
        .arg("--model")
        .arg(&model)
        .arg("--output")
        .arg(&output)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    let child = command.spawn().map_err(|error| {
        format!(
            "Could not start the optional aura-depth helper ({}): {error}",
            helper.display()
        )
    })?;
    let result = tokio::time::timeout(HELPER_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "Depth generation timed out after 5 minutes".to_string())?
        .map_err(|error| format!("Could not wait for aura-depth helper: {error}"))?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("aura-depth exited with status {}", result.status)
        } else {
            stderr
        });
    }
    if !output.is_file() {
        return Err("aura-depth completed without creating a depth map".into());
    }
    Ok(output)
}

/// Streams the ~27 MB ONNX model into the cache, reporting progress and
/// honouring cancellation. The previous model is only replaced after a
/// complete, size-checked download.
pub async fn download_model<F>(
    cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    mut progress_cb: F,
) -> Result<PathBuf, String>
where
    F: FnMut(u64, Option<u64>, f32) + Send + 'static,
{
    if !cpu_supports_avx() {
        return Err(
            "Downloading the depth model is unavailable because parallax requires a CPU with AVX support."
                .into(),
        );
    }

    let model = model_path();
    if model_present() {
        return Ok(model);
    }
    if let Some(parent) = model.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| error.to_string())?;
    }

    let partial = model.with_extension("onnx.part");
    let _ = tokio::fs::remove_file(&partial).await;

    let client = reqwest::Client::new();
    let downloaded = crate::online::cache::download_to_file_with_progress(
        &client,
        MODEL_URL,
        &partial,
        None,
        cancel_flag,
        move |downloaded, total, percent| progress_cb(downloaded, total, percent),
    )
    .await?;

    let length = tokio::fs::metadata(&downloaded)
        .await
        .map_err(|error| error.to_string())?
        .len();
    if length < 1024 * 1024 {
        let _ = tokio::fs::remove_file(&downloaded).await;
        return Err("La descarga del modelo fue demasiado pequeña".into());
    }

    tokio::fs::rename(&downloaded, &model)
        .await
        .map_err(|error| error.to_string())?;
    if std::fs::metadata(&model).map(|m| m.len()).unwrap_or(0) < 1024 * 1024 {
        return Err("El modelo descargado no es válido".into());
    }
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_path_lives_in_cache() {
        let path = model_path();
        assert!(path.to_string_lossy().contains("aura/depth/models"));
        assert!(path.to_string_lossy().ends_with(MODEL_NAME));
    }

    #[test]
    fn depth_maps_are_content_keyed() {
        let first = depth_map_path(Path::new("/tmp/aura-test-wallpaper.png"));
        let second = depth_map_path(Path::new("/tmp/aura-test-wallpaper.png"));
        assert_eq!(first, second);
    }
}
