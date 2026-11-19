//! Optional ONNX Runtime worker for depth-map generation.
//!
//! This stays in a separate executable so its ONNX Runtime static
//! initializers are never run when the main Aura application starts.

use image::{imageops::FilterType, ImageBuffer, Luma};
use ort::{environment::Environment, session::Session, value::Tensor};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const INPUT_SIZE: u32 = 518;
static ORT_ENVIRONMENT: OnceLock<Result<Environment, String>> = OnceLock::new();

fn cpu_supports_avx() -> bool {
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

fn main() {
    if !cpu_supports_avx() {
        eprintln!("aura-depth requires a CPU with AVX support");
        std::process::exit(1);
    }

    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut image = None;
    let mut model = None;
    let mut output = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(argument) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("Missing value for {}", argument.to_string_lossy()))?;
        match argument.to_string_lossy().as_ref() {
            "--image" => image = Some(PathBuf::from(value)),
            "--model" => model = Some(PathBuf::from(value)),
            "--output" => output = Some(PathBuf::from(value)),
            _ => return Err(format!("Unknown argument: {}", argument.to_string_lossy())),
        }
    }

    let image = image.ok_or_else(|| "Required argument missing: --image".to_string())?;
    let model = model.ok_or_else(|| "Required argument missing: --model".to_string())?;
    let output = output.ok_or_else(|| "Required argument missing: --output".to_string())?;
    infer_depth_map(&image, &model, &output)
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
