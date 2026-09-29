use std::path::{Path, PathBuf};
use tokio::process::Command;

pub fn get_cache_dir() -> PathBuf {
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
    base.join("aura/thumbs")
}

pub fn get_scaled_cache_dir() -> PathBuf {
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
    base.join("aura/scaled")
}

pub fn get_cinematic_cache_dir() -> PathBuf {
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
    base.join("aura/cinematic")
}

pub fn cinematic_thumb_for_media(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) -> Option<PathBuf> {
    let cache_dir = get_cinematic_cache_dir();
    let _ = std::fs::create_dir_all(&cache_dir);

    let file_stem = media_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "media".into());
    let clean_stem: String = file_stem
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(50)
        .collect();
    let hash = format!("{:016x}", md5_simple(media_path.to_string_lossy().as_bytes()));
    let suffix = if is_active { "act" } else { "inact" };
    let cached_path = cache_dir.join(format!("{}_{}_{}.png", clean_stem, &hash[..8], suffix));

    if cached_path.exists() {
        return Some(cached_path);
    }

    let std_thumb = thumb_path_for_video(media_path);
    let src_img_path = if std_thumb.exists() {
        std_thumb
    } else if media_path.exists() {
        media_path.to_path_buf()
    } else {
        return None;
    };

    let base_img = image::open(&src_img_path).ok()?;
    
    let card_w = 340.0f32;
    let card_h = 440.0f32;
    let skew = 0.32f32;
    let skew_px = (card_h * skew).round();
    let total_w = (card_w + skew_px) as u32;
    let total_h = card_h as u32;
    let radius = 16.0f32;
    let border_thick = if is_active { 3.5f32 } else { 1.5f32 };

    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let border_color = if is_active {
        [accent[0], accent[1], accent[2], 255]
    } else {
        [220, 225, 235, 90]
    };

    let scaled_src = base_img.resize_to_fill(
        card_w as u32,
        card_h as u32,
        image::imageops::FilterType::Triangle,
    ).to_rgba8();

    let mut out = image::RgbaImage::new(total_w, total_h);

    for y in 0..total_h {
        let y_f = y as f32;
        let left_edge = skew_px * (1.0 - y_f / card_h);

        for x in 0..total_w {
            let x_f = x as f32;
            let u = x_f - left_edge;
            let v = y_f;

            let d_x = (radius - u).max(0.0).max(u - (card_w - radius));
            let d_y = (radius - v).max(0.0).max(v - (card_h - radius));
            let dist = if (u < radius || u > card_w - radius) && (v < radius || v > card_h - radius) {
                (d_x * d_x + d_y * d_y).sqrt() - radius
            } else {
                (-u).max(u - card_w).max(-v).max(v - card_h)
            };

            if dist > 1.0 {
                continue;
            }

            let alpha_factor = if dist > -1.0 {
                ((1.0 - dist) * 0.5).clamp(0.0, 1.0)
            } else {
                1.0
            };

            let pixel = if dist >= -border_thick {
                let a = (border_color[3] as f32 * alpha_factor) as u8;
                image::Rgba([border_color[0], border_color[1], border_color[2], a])
            } else {
                let src_x = (u.clamp(0.0, card_w - 1.0)) as u32;
                let src_y = (v.clamp(0.0, card_h - 1.0)) as u32;
                let mut p = *scaled_src.get_pixel(src_x.min(scaled_src.width() - 1), src_y.min(scaled_src.height() - 1));
                
                if !is_active {
                    p[0] = (p[0] as f32 * 0.72) as u8;
                    p[1] = (p[1] as f32 * 0.72) as u8;
                    p[2] = (p[2] as f32 * 0.72) as u8;
                }

                p[3] = (p[3] as f32 * alpha_factor) as u8;
                p
            };

            out.put_pixel(x, y, pixel);
        }
    }

    if out.save_with_format(&cached_path, image::ImageFormat::Png).is_ok() {
        Some(cached_path)
    } else {
        None
    }
}

pub fn optimize_wallpaper_image(image_path: &Path, max_w: u32, max_h: u32) -> PathBuf {
    let (orig_w, orig_h) = match image::image_dimensions(image_path) {
        Ok(dims) => dims,
        Err(_) => return image_path.to_path_buf(),
    };

    if orig_w <= max_w && orig_h <= max_h {
        return image_path.to_path_buf();
    }

    let cache_dir = get_scaled_cache_dir();
    let _ = std::fs::create_dir_all(&cache_dir);

    let file_stem = image_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "img".into());
    let clean_stem: String = file_stem
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(50)
        .collect();
    let hash = format!("{:016x}", md5_simple(image_path.to_string_lossy().as_bytes()));

    let mtime = std::fs::metadata(image_path)
        .and_then(|m| m.modified())
        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
        .unwrap_or(0);

    let is_png = image_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("png"))
        .unwrap_or(false);
    let ext = if is_png { "png" } else { "jpg" };

    let cached_path = cache_dir.join(format!("{}_{}_{}_{}x{}.{}", clean_stem, &hash[..8], mtime, max_w, max_h, ext));
    if cached_path.exists() {
        return cached_path;
    }

    match image::open(image_path) {
        Ok(img) => {
            let scaled = img.resize(max_w, max_h, image::imageops::FilterType::Lanczos3);
            if is_png {
                if scaled.save_with_format(&cached_path, image::ImageFormat::Png).is_ok() {
                    return cached_path;
                }
            } else if let Ok(mut file) = std::fs::File::create(&cached_path) {
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 95);
                if encoder.encode_image(&scaled).is_ok() {
                    return cached_path;
                }
            }
        }
        Err(e) => {
            eprintln!("[Aura Engine] No se pudo reescalar imagen {}: {}", image_path.display(), e);
        }
    }

    image_path.to_path_buf()
}

pub fn thumb_path_for_video(video_path: &Path) -> PathBuf {
    let cache_dir = get_cache_dir();
    let file_stem = video_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "video".into());
    // Sanitize name and add hash based on full path to avoid collision
    let clean_stem: String = file_stem.chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').take(60).collect();
    let hash = format!("{:016x}", md5_simple(video_path.to_string_lossy().as_bytes()));
    cache_dir.join(format!("{}_{}.jpg", clean_stem, &hash[..8]))
}

fn md5_simple(bytes: &[u8]) -> u64 {
    // Fast lightweight FNV-1a hash for deterministic thumb names
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_simple_deterministic() {
        let h1 = md5_simple(b"/home/user/Videos/wallpaper.mp4");
        let h2 = md5_simple(b"/home/user/Videos/wallpaper.mp4");
        let h3 = md5_simple(b"/home/user/Videos/other.mp4");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_thumb_path_for_video() {
        let p = Path::new("/path/to/my video file!.mp4");
        let thumb = thumb_path_for_video(p);
        let file_name = thumb.file_name().unwrap().to_str().unwrap();
        assert!(file_name.starts_with("myvideofile_"));
        assert!(file_name.ends_with(".jpg"));
        assert!(thumb.starts_with(get_cache_dir()));
    }

    #[test]
    fn test_hash_leading_zeros_safety() {
        let hash = format!("{:016x}", 0u64);
        assert_eq!(hash.len(), 16);
        assert_eq!(&hash[..8], "00000000");
    }

    #[test]
    fn test_cinematic_thumb_generation() {
        let test_dir = std::env::temp_dir().join("aura_test_cinematic");
        let _ = std::fs::create_dir_all(&test_dir);
        let src_img_path = test_dir.join("test_sample.png");

        // Create a dummy 200x200 image
        let dummy = image::RgbaImage::from_pixel(200, 200, image::Rgba([100, 150, 200, 255]));
        dummy.save_with_format(&src_img_path, image::ImageFormat::Png).expect("save dummy");

        let generated = cinematic_thumb_for_media(&src_img_path, true, Some([255, 100, 50]));
        assert!(generated.is_some(), "Cinematic thumbnail should be generated");
        let path = generated.unwrap();
        assert!(path.exists(), "Generated cinematic thumb file must exist");
        assert!(path.to_string_lossy().ends_with("_act.png"));

        // Verify image dimensions and transparency in skewed corner
        let img = image::open(&path).expect("open generated thumb").to_rgba8();
        assert_eq!(img.width(), 481);
        assert_eq!(img.height(), 440);
        // Top-left pixel (0, 0) should be outside the tilted parallelogram (alpha == 0)
        assert_eq!(img.get_pixel(0, 0)[3], 0);

        // Clean up
        let _ = std::fs::remove_file(&src_img_path);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&test_dir);
    }
}

static THUMB_SEMAPHORE: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();

pub async fn generate_thumbnail(media_path: &Path) -> Option<PathBuf> {
    let thumb_path = thumb_path_for_video(media_path);
    if thumb_path.exists() {
        return Some(thumb_path);
    }

    let sem = THUMB_SEMAPHORE.get_or_init(|| tokio::sync::Semaphore::new(2));
    let _permit = sem.acquire().await.ok()?;

    if thumb_path.exists() {
        return Some(thumb_path);
    }

    if let Some(parent) = thumb_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    let is_image = media_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| crate::scanner::IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false);

    if is_image {
        let src = media_path.to_path_buf();
        let dst = thumb_path.clone();
        let res = tokio::task::spawn_blocking(move || {
            if let Ok(img) = image::open(&src) {
                let thumb = img.thumbnail(480, 270);
                if thumb.save(&dst).is_ok() && dst.exists() {
                    return Some(dst);
                }
            }
            None
        }).await.ok().flatten();

        if res.is_some() {
            return res;
        }
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y")
        .arg("-ss")
        .arg("00:00:01")
        .arg("-i")
        .arg(media_path)
        .arg("-an")
        .arg("-sn")
        .arg("-vframes")
        .arg("1")
        .arg("-vf")
        .arg("scale=480:-1")
        .arg(&thumb_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let status = cmd.status().await;

    match status {
        Ok(s) if s.success() && thumb_path.exists() => Some(thumb_path),
        _ => None,
    }
}
