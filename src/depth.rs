//! Local depth-map generation for mouse-following wallpaper parallax.

use image::{imageops::FilterType, ImageBuffer, Luma};
use ort::{environment::Environment, session::Session, value::Tensor};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const MODEL_URL: &str = "https://huggingface.co/onnx-community/depth-anything-v2-small/resolve/main/onnx/model_quantized.onnx";
const MODEL_NAME: &str = "depth-anything-v2-small-quantized.onnx";
const INPUT_SIZE: u32 = 518;

static ORT_ENVIRONMENT: OnceLock<Result<Environment, String>> = OnceLock::new();

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

    let output_for_work = output.clone();
    tokio::task::spawn_blocking(move || infer_depth_map(&image, &model, &output_for_work))
        .await
        .map_err(|error| format!("Depth worker failed: {error}"))??;
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

fn infer_depth_map(image_path: &Path, model_path: &Path, output_path: &Path) -> Result<(), String> {
    let environment = ORT_ENVIRONMENT
        .get_or_init(|| {
            ort::init()
                .build()
                .map_err(|error| format!("Could not initialize ONNX Runtime: {error}"))
        })
        .as_ref()
        .map_err(Clone::clone)?;
    let mut session = Session::builder(environment)
        .and_then(|mut builder| builder.commit_from_file(model_path))
        .map_err(|error| format!("Could not load depth model: {error}"))?;

    let rgb = image::open(image_path)
        .map_err(|error| format!("Could not read wallpaper image: {error}"))?
        .to_rgb8();
    let resized = image::imageops::resize(&rgb, INPUT_SIZE, INPUT_SIZE, FilterType::CatmullRom);
    let plane = (INPUT_SIZE * INPUT_SIZE) as usize;
    let mut pixels = vec![0.0f32; 3 * plane];
    let mean = [0.485f32, 0.456, 0.406];
    let std = [0.229f32, 0.224, 0.225];
    for (index, pixel) in resized.pixels().enumerate() {
        for channel in 0..3 {
            pixels[channel * plane + index] =
                (pixel[channel] as f32 / 255.0 - mean[channel]) / std[channel];
        }
    }

    let input = Tensor::from_array((vec![1_i64, 3, INPUT_SIZE as i64, INPUT_SIZE as i64], pixels))
        .map_err(|error| format!("Could not prepare depth input: {error}"))?;
    let outputs = session
        .run(ort::inputs!["pixel_values" => input])
        .map_err(|error| format!("Depth inference failed: {error}"))?;
    let (_, values) = outputs["predicted_depth"]
        .try_extract_tensor::<f32>()
        .map_err(|error| format!("Could not read depth output: {error}"))?;

    let mut finite: Vec<f32> = values.iter().copied().filter(|value| value.is_finite()).collect();
    if finite.len() != plane {
        return Err(format!(
            "Depth model returned {} valid pixels; expected {plane}",
            finite.len()
        ));
    }
    finite.sort_by(f32::total_cmp);
    let low = finite[finite.len() / 100];
    let high = finite[(finite.len() * 99 / 100).min(finite.len() - 1)];
    let range = (high - low).max(1e-6);
    let gray: Vec<u8> = values
        .iter()
        .map(|value| {
            if value.is_finite() {
                (((*value - low) / range).clamp(0.0, 1.0) * 255.0).round() as u8
            } else {
                0
            }
        })
        .collect();

    let parent = output_path
        .parent()
        .ok_or_else(|| "Invalid depth-map cache path".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let map: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_raw(INPUT_SIZE, INPUT_SIZE, gray)
        .ok_or_else(|| "Could not create depth image".to_string())?;
    let temporary = output_path.with_extension(format!("{}.tmp.png", std::process::id()));
    map.save(&temporary)
        .map_err(|error| format!("Could not save depth map: {error}"))?;
    std::fs::rename(&temporary, output_path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        error.to_string()
    })?;
    Ok(())
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
