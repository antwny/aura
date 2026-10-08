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
    base.join("aura/cinematic_v2")
}

pub fn get_honeycomb_cache_dir() -> PathBuf {
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
    base.join("aura/honeycomb_v2")
}

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{LazyLock, Mutex};
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::ImageEncoder;

#[derive(Debug)]
pub struct LruMemoryCache<K: Eq + std::hash::Hash + Clone, V: Clone> {
    capacity: usize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> LruMemoryCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            map: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(val) = self.map.get(key) {
            let val = val.clone();
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
            }
            self.order.push_back(key.clone());
            Some(val)
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: K, val: V) {
        if self.map.contains_key(&key) {
            self.map.insert(key.clone(), val);
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                self.order.remove(pos);
            }
            self.order.push_back(key);
        } else {
            while self.order.len() >= self.capacity {
                if let Some(oldest) = self.order.pop_front() {
                    self.map.remove(&oldest);
                }
            }
            self.map.insert(key.clone(), val);
            self.order.push_back(key);
        }
    }
}

pub static THUMB_HANDLE_CACHE: LazyLock<Mutex<LruMemoryCache<String, cosmic::iced::widget::image::Handle>>> =
    LazyLock::new(|| Mutex::new(LruMemoryCache::new(96)));

static CINEMATIC_IN_FLIGHT: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static HONEYCOMB_IN_FLIGHT: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

pub fn cinematic_thumb_path(media_path: &Path, is_active: bool, accent_rgb: Option<[u8; 3]>) -> PathBuf {
    let cache_dir = get_cinematic_cache_dir();
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
    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let suffix = if is_active {
        format!("act_{:02x}{:02x}{:02x}", accent[0], accent[1], accent[2])
    } else {
        "inact".to_string()
    };
    cache_dir.join(format!("{}_{}_{}.png", clean_stem, &hash[..8], suffix))
}

pub fn generate_cinematic_thumb_sync(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) -> Option<PathBuf> {
    let cache_dir = get_cinematic_cache_dir();
    let _ = std::fs::create_dir_all(&cache_dir);
    let cached_path = cinematic_thumb_path(media_path, is_active, accent_rgb);

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

            let dist = (-u).max(u - card_w).max(-v).max(v - card_h);

            if dist > 0.0 {
                if is_active && dist <= 14.0 {
                    let glow_factor = ((1.0 - dist / 14.0) * (1.0 - dist / 14.0)).clamp(0.0, 1.0);
                    let a = (170.0 * glow_factor) as u8;
                    out.put_pixel(x, y, image::Rgba([accent[0], accent[1], accent[2], a]));
                }
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

    // Direct GPU/memory cache insertion
    let handle_key = format!("cn:{}:{}:{:02x}{:02x}{:02x}", media_path.to_string_lossy(), is_active, accent[0], accent[1], accent[2]);
    let direct_handle = cosmic::iced::widget::image::Handle::from_rgba(total_w, total_h, out.as_raw().clone());
    if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
        cache.insert(handle_key, direct_handle);
    }

    let file = std::fs::File::create(&cached_path).ok()?;
    let writer = std::io::BufWriter::new(file);
    let encoder = PngEncoder::new_with_quality(
        writer,
        CompressionType::Fast,
        FilterType::NoFilter,
    );
    if encoder.write_image(
        out.as_raw(),
        total_w,
        total_h,
        image::ExtendedColorType::Rgba8,
    ).is_ok() {
        Some(cached_path)
    } else {
        None
    }
}

pub fn cinematic_thumb_handle(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) -> Option<cosmic::iced::widget::image::Handle> {
    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let key = format!("cn:{}:{}:{:02x}{:02x}{:02x}", media_path.to_string_lossy(), is_active, accent[0], accent[1], accent[2]);

    if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
        if let Some(handle) = cache.get(&key) {
            return Some(handle);
        }
    }

    let cached_path = cinematic_thumb_path(media_path, is_active, accent_rgb);
    if cached_path.exists() {
        let handle = cosmic::iced::widget::image::Handle::from_path(cached_path);
        if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
            cache.insert(key, handle.clone());
        }
        return Some(handle);
    }

    if tokio::runtime::Handle::try_current().is_ok() {
        trigger_cinematic_thumb_async(media_path.to_path_buf(), is_active, accent_rgb);
        None
    } else {
        let path = generate_cinematic_thumb_sync(media_path, is_active, accent_rgb)?;
        let handle = cosmic::iced::widget::image::Handle::from_path(path);
        if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
            cache.insert(key, handle.clone());
        }
        Some(handle)
    }
}

pub fn trigger_cinematic_thumb_async(
    media_path: PathBuf,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) {
    let cached_path = cinematic_thumb_path(&media_path, is_active, accent_rgb);
    if cached_path.exists() {
        return;
    }

    let mut in_flight = match CINEMATIC_IN_FLIGHT.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if in_flight.insert(cached_path.clone()) {
        drop(in_flight);
        tokio::task::spawn_blocking(move || {
            let _ = generate_cinematic_thumb_sync(&media_path, is_active, accent_rgb);
            if let Ok(mut in_flight) = CINEMATIC_IN_FLIGHT.lock() {
                in_flight.remove(&cached_path);
            }
        });
    }
}

pub fn prewarm_cinematic_thumbs(media_paths: &[PathBuf], accent_rgb: [u8; 3]) {
    for path in media_paths {
        trigger_cinematic_thumb_async(path.clone(), true, Some(accent_rgb));
        trigger_cinematic_thumb_async(path.clone(), false, Some(accent_rgb));
    }
}

pub fn cinematic_thumb_for_media(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) -> Option<PathBuf> {
    let cached_path = cinematic_thumb_path(media_path, is_active, accent_rgb);
    if cached_path.exists() {
        return Some(cached_path);
    }

    // In asynchronous tokio environment (UI app runtime), never block the frame.
    // Trigger background generation and return None so fallback thumbnail or placeholder renders at 60 FPS.
    if tokio::runtime::Handle::try_current().is_ok() {
        trigger_cinematic_thumb_async(media_path.to_path_buf(), is_active, accent_rgb);
        None
    } else {
        // Synchronous fallback (e.g. CLI or unit tests without tokio runtime)
        generate_cinematic_thumb_sync(media_path, is_active, accent_rgb)
    }
}

pub fn honeycomb_thumb_path(media_path: &Path, is_active: bool, accent_rgb: Option<[u8; 3]>) -> PathBuf {
    let cache_dir = get_honeycomb_cache_dir();
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
    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let suffix = if is_active {
        format!("act_{:02x}{:02x}{:02x}", accent[0], accent[1], accent[2])
    } else {
        "inact".to_string()
    };
    cache_dir.join(format!("{}_{}_{}.png", clean_stem, &hash[..8], suffix))
}

pub fn generate_honeycomb_thumb_sync(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) -> Option<PathBuf> {
    let cache_dir = get_honeycomb_cache_dir();
    let _ = std::fs::create_dir_all(&cache_dir);
    let cached_path = honeycomb_thumb_path(media_path, is_active, accent_rgb);

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
    let card_w = 260.0f32;
    let card_h = (card_w * 1.1547005f32).round();
    let total_w = card_w as u32;
    let total_h = card_h as u32;

    let inradius = card_w * 0.5 - 8.0;
    let corner_r = 8.0f32;
    let border_thick = if is_active { 4.5f32 } else { 2.0f32 };

    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let border_color = if is_active {
        [accent[0], accent[1], accent[2], 255]
    } else {
        [220, 228, 240, 110]
    };

    let scaled_src = base_img.resize_to_fill(
        total_w,
        total_h,
        image::imageops::FilterType::Triangle,
    ).to_rgba8();

    let mut out = image::RgbaImage::new(total_w, total_h);

    let cx = card_w * 0.5;
    let cy = card_h * 0.5;
    let r_inner = inradius - corner_r;
    let k_x = -0.866025404f32;
    let k_y = 0.5f32;
    let k_z = 0.577350269f32;

    // Exact signed distance function for a pointy-topped hexagon (flat vertical sides, points at top and bottom)
    let sd_pointy_hex = |p_x: f32, p_y: f32, inr: f32| -> f32 {
        let mut x = p_y.abs();
        let mut y = p_x.abs();
        let dot = 2.0 * (k_x * x + k_y * y).min(0.0);
        x -= dot * k_x;
        y -= dot * k_y;
        let d_x = x.clamp(-k_z * inr, k_z * inr);
        let d_y = inr;
        let vx = x - d_x;
        let vy = y - d_y;
        (vx * vx + vy * vy).sqrt().copysign(if y > inr { 1.0 } else { -1.0 })
    };

    for y in 0..total_h {
        let py = y as f32 - cy;
        for x in 0..total_w {
            let px = x as f32 - cx;
            let dist = sd_pointy_hex(px, py, r_inner) - corner_r;

            if dist > 0.0 {
                if is_active && dist <= 8.0 {
                    let glow_factor = ((1.0 - dist / 8.0) * (1.0 - dist / 8.0)).clamp(0.0, 1.0);
                    let a = (220.0 * glow_factor) as u8;
                    out.put_pixel(x, y, image::Rgba([accent[0], accent[1], accent[2], a]));
                }
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
                let mut p = *scaled_src.get_pixel(x, y);
                if !is_active {
                    p[0] = (p[0] as f32 * 0.88) as u8;
                    p[1] = (p[1] as f32 * 0.88) as u8;
                    p[2] = (p[2] as f32 * 0.88) as u8;
                }
                p[3] = (p[3] as f32 * alpha_factor) as u8;
                p
            };

            out.put_pixel(x, y, pixel);
        }
    }

    // Direct GPU/memory cache insertion
    let handle_key = format!("hc:{}:{}:{:02x}{:02x}{:02x}", media_path.to_string_lossy(), is_active, accent[0], accent[1], accent[2]);
    let direct_handle = cosmic::iced::widget::image::Handle::from_rgba(total_w, total_h, out.as_raw().clone());
    if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
        cache.insert(handle_key, direct_handle);
    }

    let file = std::fs::File::create(&cached_path).ok()?;
    let writer = std::io::BufWriter::new(file);
    let encoder = PngEncoder::new_with_quality(
        writer,
        CompressionType::Fast,
        FilterType::NoFilter,
    );
    if encoder.write_image(
        out.as_raw(),
        total_w,
        total_h,
        image::ExtendedColorType::Rgba8,
    ).is_ok() {
        Some(cached_path)
    } else {
        None
    }
}

pub fn honeycomb_thumb_handle(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) -> Option<cosmic::iced::widget::image::Handle> {
    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let key = format!("hc:{}:{}:{:02x}{:02x}{:02x}", media_path.to_string_lossy(), is_active, accent[0], accent[1], accent[2]);

    if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
        if let Some(handle) = cache.get(&key) {
            return Some(handle);
        }
    }

    let cached_path = honeycomb_thumb_path(media_path, is_active, accent_rgb);
    if cached_path.exists() {
        let handle = cosmic::iced::widget::image::Handle::from_path(cached_path);
        if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
            cache.insert(key, handle.clone());
        }
        return Some(handle);
    }

    if tokio::runtime::Handle::try_current().is_ok() {
        trigger_honeycomb_thumb_async(media_path.to_path_buf(), is_active, accent_rgb);
        None
    } else {
        let path = generate_honeycomb_thumb_sync(media_path, is_active, accent_rgb)?;
        let handle = cosmic::iced::widget::image::Handle::from_path(path);
        if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
            cache.insert(key, handle.clone());
        }
        Some(handle)
    }
}

pub fn trigger_honeycomb_thumb_async(
    media_path: PathBuf,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) {
    let cached_path = honeycomb_thumb_path(&media_path, is_active, accent_rgb);
    if cached_path.exists() {
        return;
    }

    let mut in_flight = match HONEYCOMB_IN_FLIGHT.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if in_flight.insert(cached_path.clone()) {
        drop(in_flight);
        tokio::task::spawn_blocking(move || {
            let _ = generate_honeycomb_thumb_sync(&media_path, is_active, accent_rgb);
            if let Ok(mut in_flight) = HONEYCOMB_IN_FLIGHT.lock() {
                in_flight.remove(&cached_path);
            }
        });
    }
}

pub fn prewarm_honeycomb_thumbs(media_paths: &[PathBuf], accent_rgb: [u8; 3]) {
    for path in media_paths {
        trigger_honeycomb_thumb_async(path.clone(), true, Some(accent_rgb));
        trigger_honeycomb_thumb_async(path.clone(), false, Some(accent_rgb));
    }
}

pub fn honeycomb_thumb_for_media(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
) -> Option<PathBuf> {
    let cached_path = honeycomb_thumb_path(media_path, is_active, accent_rgb);
    if cached_path.exists() {
        return Some(cached_path);
    }

    if tokio::runtime::Handle::try_current().is_ok() {
        trigger_honeycomb_thumb_async(media_path.to_path_buf(), is_active, accent_rgb);
        None
    } else {
        generate_honeycomb_thumb_sync(media_path, is_active, accent_rgb)
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
        assert!(path.to_string_lossy().ends_with("_act_ff6432.png"));

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

    #[test]
    fn test_honeycomb_thumb_generation() {
        let test_dir = std::env::temp_dir().join("aura_test_honeycomb");
        let _ = std::fs::create_dir_all(&test_dir);
        let src_img_path = test_dir.join("test_hex_sample.png");

        let dummy = image::RgbaImage::from_pixel(200, 200, image::Rgba([120, 180, 240, 255]));
        dummy.save_with_format(&src_img_path, image::ImageFormat::Png).expect("save dummy hex");

        let generated = honeycomb_thumb_for_media(&src_img_path, true, Some([255, 100, 50]));
        assert!(generated.is_some(), "Honeycomb thumbnail should be generated");
        let path = generated.unwrap();
        assert!(path.to_string_lossy().ends_with("_act_ff6432.png"));

        let img = image::open(&path).expect("open generated hex thumb").to_rgba8();
        assert_eq!(img.width(), 260);
        assert_eq!(img.height(), 300);
        // Corner (0, 0) of pointy-topped hexagon must be transparent (alpha == 0)
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        // Center pixel should be opaque (alpha == 255)
        assert_eq!(img.get_pixel(130, 150)[3], 255);

        // Clean up
        let _ = std::fs::remove_file(&src_img_path);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_lru_memory_cache() {
        let mut cache = LruMemoryCache::<String, usize>::new(3);
        cache.insert("a".into(), 1);
        cache.insert("b".into(), 2);
        cache.insert("c".into(), 3);
        assert_eq!(cache.get(&"a".into()), Some(1));
        // Inserting "d" should evict "b" since "a" was recently accessed
        cache.insert("d".into(), 4);
        assert_eq!(cache.get(&"b".into()), None);
        assert_eq!(cache.get(&"a".into()), Some(1));
        assert_eq!(cache.get(&"c".into()), Some(3));
        assert_eq!(cache.get(&"d".into()), Some(4));
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
