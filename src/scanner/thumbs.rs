use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Condvar, LazyLock, Mutex};

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::ImageEncoder;

fn cache_base_dir() -> PathBuf {
    #[cfg(test)]
    crate::test_support::init();

    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".cache")
}

pub fn get_cache_dir() -> PathBuf {
    cache_base_dir().join("aura/thumbs")
}

pub fn get_scaled_cache_dir() -> PathBuf {
    cache_base_dir().join("aura/scaled")
}

fn get_poster_cache_dir() -> PathBuf {
    cache_base_dir().join("aura/posters")
}

pub fn get_cinematic_cache_dir() -> PathBuf {
    cache_base_dir().join("aura/cinematic_v3")
}

pub fn get_honeycomb_cache_dir() -> PathBuf {
    cache_base_dir().join("aura/honeycomb_v3")
}

pub fn get_classic_cache_dir() -> PathBuf {
    cache_base_dir().join("aura/classic_v1")
}

/// Small generic LRU retained for tests and future non-texture caches.
#[cfg(test)]
#[derive(Debug)]
pub struct LruMemoryCache<K: Eq + std::hash::Hash + Clone, V: Clone> {
    capacity: usize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
}

#[cfg(test)]
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

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

/// Byte-weighted LRU for decoded GPU textures. 2x HUD assets are several MiB
/// each; an item-count cache either thrashes or holds hundreds of MiB. The
/// budget here is deliberately conservative and is also a hard ceiling for the
/// prewarm window.
#[derive(Debug)]
pub struct TextureCache {
    max_bytes: usize,
    bytes: usize,
    map: HashMap<String, (cosmic::iced::widget::image::Handle, usize)>,
    order: VecDeque<String>,
}

impl TextureCache {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes: max_bytes.max(1),
            bytes: 0,
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<cosmic::iced::widget::image::Handle> {
        let handle = self.map.get(key)?.0.clone();
        if let Some(pos) = self.order.iter().position(|candidate| candidate == key) {
            self.order.remove(pos);
        }
        self.order.push_back(key.to_string());
        Some(handle)
    }

    pub fn insert(
        &mut self,
        key: String,
        handle: cosmic::iced::widget::image::Handle,
        width: u32,
        height: u32,
    ) {
        let weight = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4)
            .saturating_add(2048);
        if let Some((_, old_weight)) = self.map.remove(&key) {
            self.bytes = self.bytes.saturating_sub(old_weight);
        }
        if let Some(pos) = self.order.iter().position(|candidate| candidate == &key) {
            self.order.remove(pos);
        }
        while self.bytes.saturating_add(weight) > self.max_bytes {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some((_, old_weight)) = self.map.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(old_weight);
            }
        }
        self.bytes = self.bytes.saturating_add(weight);
        self.map.insert(key.clone(), (handle, weight));
        self.order.push_back(key);
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
        self.bytes = 0;
    }

    #[cfg(test)]
    pub fn used_bytes(&self) -> usize {
        self.bytes
    }
}

pub static THUMB_HANDLE_CACHE: LazyLock<Mutex<TextureCache>> =
    LazyLock::new(|| Mutex::new(TextureCache::new(192 * 1024 * 1024)));

/// Trims in-memory texture handles and requests glibc to release unused heap pages back to the OS.
pub fn trim_memory() {
    if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
        cache.clear();
    }
    #[cfg(target_os = "linux")]
    unsafe {
        libc::malloc_trim(0);
    }
}

static CINEMATIC_IN_FLIGHT: LazyLock<Mutex<HashSet<PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static HONEYCOMB_IN_FLIGHT: LazyLock<Mutex<HashSet<PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static CLASSIC_IN_FLIGHT: LazyLock<Mutex<HashSet<PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static POSTER_IN_FLIGHT: LazyLock<Mutex<HashSet<PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Master posters are decoded from 4K sources. Limit concurrent ffmpeg
/// processes so opening the HUD cannot pin every core while it prewarms.
static POSTER_SLOTS: LazyLock<(Mutex<usize>, Condvar)> =
    LazyLock::new(|| (Mutex::new(0), Condvar::new()));
const POSTER_MAX_CONCURRENCY: usize = 2;

struct PosterSlot;

impl PosterSlot {
    fn acquire() -> Self {
        let (lock, condition) = &*POSTER_SLOTS;
        let mut running = lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        while *running >= POSTER_MAX_CONCURRENCY {
            running = condition
                .wait(running)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        *running += 1;
        Self
    }
}

impl Drop for PosterSlot {
    fn drop(&mut self) {
        let (lock, condition) = &*POSTER_SLOTS;
        if let Ok(mut running) = lock.lock() {
            *running = running.saturating_sub(1);
        }
        condition.notify_one();
    }
}

fn clean_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "media".into())
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(50)
        .collect()
}

fn path_hash(path: &Path) -> String {
    let key = format!("{}:{}", path.to_string_lossy(), source_stamp(path));
    format!("{:016x}", md5_simple(key.as_bytes()))
}

fn source_stamp(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map(|time| {
            time.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        })
        .unwrap_or(0)
}

fn asset_name(path: &Path, suffix: &str) -> String {
    format!(
        "{}_{}_{}.png",
        clean_stem(path),
        &path_hash(path)[..8],
        suffix
    )
}

fn scale_suffix(scale: u8) -> String {
    if scale <= 1 {
        String::new()
    } else {
        format!("_s{}", scale)
    }
}

fn accent_suffix(is_active: bool, accent_rgb: Option<[u8; 3]>, scale: u8) -> String {
    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    if is_active {
        format!(
            "act_{:02x}{:02x}{:02x}{}",
            accent[0],
            accent[1],
            accent[2],
            scale_suffix(scale)
        )
    } else {
        format!("inact{}", scale_suffix(scale))
    }
}

pub fn cinematic_thumb_path(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
    scale: u8,
) -> PathBuf {
    get_cinematic_cache_dir().join(asset_name(
        media_path,
        &accent_suffix(is_active, accent_rgb, scale),
    ))
}

pub fn honeycomb_thumb_path(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
    scale: u8,
) -> PathBuf {
    get_honeycomb_cache_dir().join(asset_name(
        media_path,
        &accent_suffix(is_active, accent_rgb, scale),
    ))
}

pub fn classic_thumb_path(media_path: &Path, scale: u8) -> PathBuf {
    get_classic_cache_dir().join(asset_name(
        media_path,
        &format!("classic{}", scale_suffix(scale)),
    ))
}

/// High-resolution poster cache used as the source for every HUD asset and as
/// the static fallback when a video backend is unavailable. Only videos need a
/// poster file: images are decoded from their original file.
pub fn master_poster_path(media_path: &Path) -> PathBuf {
    let stamp = source_stamp(media_path);
    get_poster_cache_dir().join(format!(
        "{}_{}_{}.jpg",
        clean_stem(media_path),
        &path_hash(media_path)[..8],
        stamp
    ))
}

fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| crate::scanner::IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn atomic_temp_path(destination: &Path, extension: &str) -> PathBuf {
    let unique = format!(
        ".tmp-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        md5_simple(destination.to_string_lossy().as_bytes())
    );
    destination.with_file_name(format!("{}{}.{}", unique, destination.file_name().unwrap_or_default().to_string_lossy(), extension))
}

fn atomic_save_png(destination: &Path, rgba: &image::RgbaImage) -> Option<()> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).ok()?;
    }
    let temp = atomic_temp_path(destination, "png");
    let file = std::fs::File::create(&temp).ok()?;
    let writer = std::io::BufWriter::new(file);
    let encoder = PngEncoder::new_with_quality(writer, CompressionType::Fast, FilterType::NoFilter);
    let result = encoder.write_image(
        rgba.as_raw(),
        rgba.width(),
        rgba.height(),
        image::ExtendedColorType::Rgba8,
    );
    match result {
        Ok(()) => {
            if std::fs::rename(&temp, destination).is_ok() {
                Some(())
            } else {
                let _ = std::fs::remove_file(&temp);
                None
            }
        }
        Err(_) => {
            let _ = std::fs::remove_file(&temp);
            None
        }
    }
}

/// Generates a 1280px-wide still from a video. The poster is written atomically
/// so a cancelled or concurrent generation can never leave a half JPEG that a
/// later run would trust.
fn generate_master_poster(media_path: &Path) -> Option<PathBuf> {
    let destination = master_poster_path(media_path);
    if destination.exists() {
        return Some(destination);
    }
    if is_image_path(media_path) {
        return Some(media_path.to_path_buf());
    }
    let in_flight_key = destination.clone();
    {
        let mut in_flight = POSTER_IN_FLIGHT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !in_flight.insert(in_flight_key.clone()) {
            drop(in_flight);
            // Another HUD asset is already extracting the shared poster. Wait
            // for it instead of silently generating a permanently-lower-res
            // card from the 480px library thumbnail.
            for _ in 0..60 {
                if destination.exists() {
                    return Some(destination);
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            return None;
        }
    }
    if let Some(parent) = destination.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let temp = atomic_temp_path(&destination, "jpg");
    let _poster_slot = PosterSlot::acquire();
    let output = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss")
        .arg("00:00:01")
        .arg("-i")
        .arg(media_path)
        .arg("-an")
        .arg("-sn")
        .arg("-vframes")
        .arg("1")
        .arg("-vf")
        .arg("scale=1280:-2:flags=lanczos")
        .arg("-q:v")
        .arg("2")
        .arg(&temp)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if let Ok(mut in_flight) = POSTER_IN_FLIGHT.lock() {
        in_flight.remove(&in_flight_key);
    }
    if output && temp.exists() {
        if std::fs::rename(&temp, &destination).is_ok() {
            return Some(destination);
        }
    }
    let _ = std::fs::remove_file(&temp);
    None
}

/// Returns a still image for any supported wallpaper. This is used by the KDE
/// fallback, GNOME static mode and Static backend: a video without its native
/// plugin still gets a representative frame instead of an error-only state.
pub fn poster_for_media(media_path: &Path) -> Option<PathBuf> {
    if !media_path.exists() {
        return None;
    }
    if is_image_path(media_path) {
        return Some(media_path.to_path_buf());
    }
    if let Some(poster) = generate_master_poster(media_path) {
        return Some(poster);
    }
    let thumb = thumb_path_for_video(media_path);
    if thumb.exists() {
        return Some(thumb);
    }
    generate_thumb_sync(media_path)
}

/// Loads the best available source for HUD artwork: original images, the
/// high-resolution poster for videos, and finally the 480px library thumbnail.
fn hud_source_image(media_path: &Path) -> Option<image::DynamicImage> {
    if !media_path.exists() {
        return None;
    }
    if is_image_path(media_path) {
        return image::open(media_path).ok();
    }
    if let Some(poster) = generate_master_poster(media_path) {
        if let Ok(image) = image::open(&poster) {
            return Some(image);
        }
    }
    let thumb = thumb_path_for_video(media_path);
    if thumb.exists() {
        return image::open(thumb).ok();
    }
    None
}

pub fn generate_cinematic_thumb_sync(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
    scale: u8,
) -> Option<PathBuf> {
    let scale = scale.max(1);
    let cached_path = cinematic_thumb_path(media_path, is_active, accent_rgb, scale);
    if cached_path.exists() {
        return Some(cached_path);
    }

    let base_img = hud_source_image(media_path)?;

    let card_w = 340.0f32 * scale as f32;
    let card_h = 440.0f32 * scale as f32;
    let skew = 0.32f32;
    let skew_px = (card_h * skew).round();
    let total_w = (card_w + skew_px) as u32;
    let total_h = card_h as u32;
    let border_thick = if is_active { 3.5 * scale as f32 } else { 1.5 * scale as f32 };

    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let border_color = if is_active {
        [accent[0], accent[1], accent[2], 255]
    } else {
        [220, 225, 235, 90]
    };

    let scaled_src = base_img
        .resize_to_fill(
            card_w as u32,
            card_h as u32,
            image::imageops::FilterType::Lanczos3,
        )
        .to_rgba8();

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
                let glow_radius = 14.0 * scale as f32;
                if is_active && dist <= glow_radius {
                    let glow_factor = ((1.0 - dist / glow_radius)
                        * (1.0 - dist / glow_radius))
                        .clamp(0.0, 1.0);
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
                let mut p = *scaled_src.get_pixel(
                    src_x.min(scaled_src.width() - 1),
                    src_y.min(scaled_src.height() - 1),
                );
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

    let handle_key = format!(
        "cn:{}:{}:{:02x}{:02x}{:02x}:{}",
        media_path.to_string_lossy(),
        is_active,
        accent[0],
        accent[1],
        accent[2],
        scale
    );
    let direct_handle =
        cosmic::iced::widget::image::Handle::from_rgba(total_w, total_h, out.as_raw().clone());
    if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
        cache.insert(handle_key, direct_handle, total_w, total_h);
    }

    atomic_save_png(&cached_path, &out)?;
    Some(cached_path)
}

pub fn cinematic_thumb_handle(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
    scale: u8,
) -> Option<cosmic::iced::widget::image::Handle> {
    let scale = scale.max(1);
    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let key = format!(
        "cn:{}:{}:{:02x}{:02x}{:02x}:{}",
        media_path.to_string_lossy(),
        is_active,
        accent[0],
        accent[1],
        accent[2],
        scale
    );

    if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
        if let Some(handle) = cache.get(&key) {
            return Some(handle);
        }
    }

    let cached_path = cinematic_thumb_path(media_path, is_active, accent_rgb, scale);
    if cached_path.exists() {
        let handle = cosmic::iced::widget::image::Handle::from_path(cached_path);
        if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
            let (w, h) = cinematic_dimensions(scale);
            cache.insert(key, handle.clone(), w, h);
        }
        return Some(handle);
    }

    if tokio::runtime::Handle::try_current().is_ok() {
        trigger_cinematic_thumb_async(media_path.to_path_buf(), is_active, accent_rgb, scale);
        None
    } else {
        let path = generate_cinematic_thumb_sync(media_path, is_active, accent_rgb, scale)?;
        let handle = cosmic::iced::widget::image::Handle::from_path(path);
        if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
            let (w, h) = cinematic_dimensions(scale);
            cache.insert(key, handle.clone(), w, h);
        }
        Some(handle)
    }
}

fn cinematic_dimensions(scale: u8) -> (u32, u32) {
    let card_w = 340.0f32 * scale as f32;
    let card_h = 440.0f32 * scale as f32;
    ((card_w + (card_h * 0.32).round()) as u32, card_h as u32)
}

pub fn trigger_cinematic_thumb_async(
    media_path: PathBuf,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
    scale: u8,
) {
    let scale = scale.max(1);
    let cached_path = cinematic_thumb_path(&media_path, is_active, accent_rgb, scale);
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
            let _ = generate_cinematic_thumb_sync(&media_path, is_active, accent_rgb, scale);
            if let Ok(mut in_flight) = CINEMATIC_IN_FLIGHT.lock() {
                in_flight.remove(&cached_path);
            }
        });
    }
}

/// Prewarms neutral inactive cards around the selection and only the accent
/// variant of the card that is actually focused. Accent assets are the
/// expensive ones, so scrolling through a large library no longer produces a
/// new accent PNG for every visited neighbor.
pub fn prewarm_cinematic_thumbs(
    media_paths: &[PathBuf],
    active_path: Option<&Path>,
    accent_rgb: [u8; 3],
    scale: u8,
) {
    for path in media_paths {
        let is_active = active_path.map(|active| active == path).unwrap_or(false);
        trigger_cinematic_thumb_async(path.clone(), is_active, Some(accent_rgb), scale);
    }
}

pub fn cinematic_thumb_for_media(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
    scale: u8,
) -> Option<PathBuf> {
    let scale = scale.max(1);
    let cached_path = cinematic_thumb_path(media_path, is_active, accent_rgb, scale);
    if cached_path.exists() {
        return Some(cached_path);
    }

    if tokio::runtime::Handle::try_current().is_ok() {
        trigger_cinematic_thumb_async(media_path.to_path_buf(), is_active, accent_rgb, scale);
        None
    } else {
        generate_cinematic_thumb_sync(media_path, is_active, accent_rgb, scale)
    }
}

pub fn generate_honeycomb_thumb_sync(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
    scale: u8,
) -> Option<PathBuf> {
    let scale = scale.max(1);
    let cached_path = honeycomb_thumb_path(media_path, is_active, accent_rgb, scale);
    if cached_path.exists() {
        return Some(cached_path);
    }

    let base_img = hud_source_image(media_path)?;
    let card_w = 260.0f32 * scale as f32;
    let card_h = (card_w * 1.1547005f32).round();
    let total_w = card_w as u32;
    let total_h = card_h as u32;

    let inradius = card_w * 0.5 - 8.0 * scale as f32;
    let corner_r = 8.0f32 * scale as f32;
    let border_thick = if is_active { 4.5 * scale as f32 } else { 2.0 * scale as f32 };

    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let border_color = if is_active {
        [accent[0], accent[1], accent[2], 255]
    } else {
        [220, 228, 240, 110]
    };

    let scaled_src = base_img
        .resize_to_fill(total_w, total_h, image::imageops::FilterType::Lanczos3)
        .to_rgba8();

    let mut out = image::RgbaImage::new(total_w, total_h);

    let cx = card_w * 0.5;
    let cy = card_h * 0.5;
    let r_inner = inradius - corner_r;
    let k_x = -0.866025404f32;
    let k_y = 0.5f32;
    let k_z = 0.577350269f32;

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
        (vx * vx + vy * vy)
            .sqrt()
            .copysign(if y > inr { 1.0 } else { -1.0 })
    };

    for y in 0..total_h {
        let py = y as f32 - cy;
        for x in 0..total_w {
            let px = x as f32 - cx;
            let dist = sd_pointy_hex(px, py, r_inner) - corner_r;

            if dist > 0.0 {
                let glow_radius = 8.0 * scale as f32;
                if is_active && dist <= glow_radius {
                    let glow_factor = ((1.0 - dist / glow_radius)
                        * (1.0 - dist / glow_radius))
                        .clamp(0.0, 1.0);
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

    let handle_key = format!(
        "hc:{}:{}:{:02x}{:02x}{:02x}:{}",
        media_path.to_string_lossy(),
        is_active,
        accent[0],
        accent[1],
        accent[2],
        scale
    );
    let direct_handle =
        cosmic::iced::widget::image::Handle::from_rgba(total_w, total_h, out.as_raw().clone());
    if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
        cache.insert(handle_key, direct_handle, total_w, total_h);
    }

    atomic_save_png(&cached_path, &out)?;
    Some(cached_path)
}

pub fn honeycomb_thumb_handle(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
    scale: u8,
) -> Option<cosmic::iced::widget::image::Handle> {
    let scale = scale.max(1);
    let accent = accent_rgb.unwrap_or([58, 142, 230]);
    let key = format!(
        "hc:{}:{}:{:02x}{:02x}{:02x}:{}",
        media_path.to_string_lossy(),
        is_active,
        accent[0],
        accent[1],
        accent[2],
        scale
    );

    if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
        if let Some(handle) = cache.get(&key) {
            return Some(handle);
        }
    }

    let cached_path = honeycomb_thumb_path(media_path, is_active, accent_rgb, scale);
    if cached_path.exists() {
        let handle = cosmic::iced::widget::image::Handle::from_path(cached_path);
        if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
            let (w, h) = honeycomb_dimensions(scale);
            cache.insert(key, handle.clone(), w, h);
        }
        return Some(handle);
    }

    if tokio::runtime::Handle::try_current().is_ok() {
        trigger_honeycomb_thumb_async(media_path.to_path_buf(), is_active, accent_rgb, scale);
        None
    } else {
        let path = generate_honeycomb_thumb_sync(media_path, is_active, accent_rgb, scale)?;
        let handle = cosmic::iced::widget::image::Handle::from_path(path);
        if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
            let (w, h) = honeycomb_dimensions(scale);
            cache.insert(key, handle.clone(), w, h);
        }
        Some(handle)
    }
}

fn honeycomb_dimensions(scale: u8) -> (u32, u32) {
    let card_w = 260.0f32 * scale as f32;
    (card_w as u32, (card_w * 1.1547005).round() as u32)
}

pub fn trigger_honeycomb_thumb_async(
    media_path: PathBuf,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
    scale: u8,
) {
    let scale = scale.max(1);
    let cached_path = honeycomb_thumb_path(&media_path, is_active, accent_rgb, scale);
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
            let _ = generate_honeycomb_thumb_sync(&media_path, is_active, accent_rgb, scale);
            if let Ok(mut in_flight) = HONEYCOMB_IN_FLIGHT.lock() {
                in_flight.remove(&cached_path);
            }
        });
    }
}

pub fn prewarm_honeycomb_thumbs(
    media_paths: &[PathBuf],
    active_path: Option<&Path>,
    accent_rgb: [u8; 3],
    scale: u8,
) {
    for path in media_paths {
        let is_active = active_path.map(|active| active == path).unwrap_or(false);
        if is_active {
            trigger_honeycomb_thumb_async(path.clone(), true, Some(accent_rgb), scale);
        } else {
            trigger_honeycomb_thumb_async(path.clone(), false, Some(accent_rgb), scale);
        }
    }
}

pub fn honeycomb_thumb_for_media(
    media_path: &Path,
    is_active: bool,
    accent_rgb: Option<[u8; 3]>,
    scale: u8,
) -> Option<PathBuf> {
    let scale = scale.max(1);
    let cached_path = honeycomb_thumb_path(media_path, is_active, accent_rgb, scale);
    if cached_path.exists() {
        return Some(cached_path);
    }

    if tokio::runtime::Handle::try_current().is_ok() {
        trigger_honeycomb_thumb_async(media_path.to_path_buf(), is_active, accent_rgb, scale);
        None
    } else {
        generate_honeycomb_thumb_sync(media_path, is_active, accent_rgb, scale)
    }
}

/// Classic cards are not skewed and do not depend on the accent color. There is
/// therefore exactly one neutral asset per scale, which keeps memory and disk
/// use low while still giving HiDPI displays a native-resolution source.
pub fn generate_classic_thumb_sync(media_path: &Path, scale: u8) -> Option<PathBuf> {
    let scale = scale.max(1);
    let cached_path = classic_thumb_path(media_path, scale);
    if cached_path.exists() {
        return Some(cached_path);
    }
    let base_img = hud_source_image(media_path)?;
    let (width, height) = (300 * scale as u32, 188 * scale as u32);
    let resized = base_img
        .resize_to_fill(width, height, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    atomic_save_png(&cached_path, &resized)?;
    Some(cached_path)
}

pub fn classic_thumb_handle(
    media_path: &Path,
    scale: u8,
) -> Option<cosmic::iced::widget::image::Handle> {
    let scale = scale.max(1);
    let key = format!("cl:{}:{}", media_path.to_string_lossy(), scale);
    if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
        if let Some(handle) = cache.get(&key) {
            return Some(handle);
        }
    }
    let cached_path = classic_thumb_path(media_path, scale);
    if cached_path.exists() {
        let handle = cosmic::iced::widget::image::Handle::from_path(cached_path);
        if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
            cache.insert(key, handle.clone(), 300 * scale as u32, 188 * scale as u32);
        }
        return Some(handle);
    }
    if tokio::runtime::Handle::try_current().is_ok() {
        trigger_classic_thumb_async(media_path.to_path_buf(), scale);
        None
    } else {
        let path = generate_classic_thumb_sync(media_path, scale)?;
        let handle = cosmic::iced::widget::image::Handle::from_path(path);
        if let Ok(mut cache) = THUMB_HANDLE_CACHE.lock() {
            cache.insert(key, handle.clone(), 300 * scale as u32, 188 * scale as u32);
        }
        Some(handle)
    }
}

pub fn trigger_classic_thumb_async(media_path: PathBuf, scale: u8) {
    let scale = scale.max(1);
    let cached_path = classic_thumb_path(&media_path, scale);
    if cached_path.exists() {
        return;
    }
    let mut in_flight = match CLASSIC_IN_FLIGHT.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if in_flight.insert(cached_path.clone()) {
        drop(in_flight);
        tokio::task::spawn_blocking(move || {
            let _ = generate_classic_thumb_sync(&media_path, scale);
            if let Ok(mut in_flight) = CLASSIC_IN_FLIGHT.lock() {
                in_flight.remove(&cached_path);
            }
        });
    }
}

pub fn prewarm_classic_thumbs(media_paths: &[PathBuf], scale: u8) {
    for path in media_paths {
        trigger_classic_thumb_async(path.clone(), scale);
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

    let mtime = source_stamp(image_path);

    let is_png = image_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("png"))
        .unwrap_or(false);
    let ext = if is_png { "png" } else { "jpg" };

    let cached_path = cache_dir.join(format!(
        "{}_{}_{}_{}x{}.{}",
        clean_stem, &hash[..8], mtime, max_w, max_h, ext
    ));
    if cached_path.exists() {
        return cached_path;
    }

    match image::open(image_path) {
        Ok(img) => {
            let scaled = img.resize(max_w, max_h, image::imageops::FilterType::Lanczos3);
            if is_png {
                if scaled
                    .save_with_format(&cached_path, image::ImageFormat::Png)
                    .is_ok()
                {
                    return cached_path;
                }
            } else if let Ok(mut file) = std::fs::File::create(&cached_path) {
                let mut encoder =
                    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 95);
                if encoder.encode_image(&scaled).is_ok() {
                    return cached_path;
                }
            }
        }
        Err(e) => {
            eprintln!(
                "[Aura Engine] No se pudo reescalar imagen {}: {}",
                image_path.display(),
                e
            );
        }
    }

    image_path.to_path_buf()
}

pub fn thumb_path_for_video(video_path: &Path) -> PathBuf {
    let cache_dir = get_cache_dir();
    let file_stem = video_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "video".into());
    let clean_stem: String = file_stem
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(60)
        .collect();
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

        let dummy = image::RgbaImage::from_pixel(200, 200, image::Rgba([100, 150, 200, 255]));
        dummy
            .save_with_format(&src_img_path, image::ImageFormat::Png)
            .expect("save dummy");

        let generated = cinematic_thumb_for_media(&src_img_path, true, Some([255, 100, 50]), 1);
        assert!(generated.is_some(), "Cinematic thumbnail should be generated");
        let path = generated.unwrap();
        assert!(path.to_string_lossy().ends_with("_act_ff6432.png"));

        let img = image::open(&path).expect("open generated thumb").to_rgba8();
        assert_eq!(img.width(), 481);
        assert_eq!(img.height(), 440);
        assert_eq!(img.get_pixel(0, 0)[3], 0);

        let _ = std::fs::remove_file(&src_img_path);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_cinematic_thumb_scale_bucket() {
        let test_dir = std::env::temp_dir().join("aura_test_cinematic_2x");
        let _ = std::fs::create_dir_all(&test_dir);
        let src_img_path = test_dir.join("test_sample_2x.png");

        let dummy = image::RgbaImage::from_pixel(200, 200, image::Rgba([100, 150, 200, 255]));
        dummy
            .save_with_format(&src_img_path, image::ImageFormat::Png)
            .expect("save dummy");

        let generated = cinematic_thumb_for_media(&src_img_path, false, Some([255, 100, 50]), 2);
        let path = generated.expect("2x cinematic thumbnail");
        assert!(path.to_string_lossy().ends_with("_inact_s2.png"));

        let img = image::open(&path).expect("open 2x thumb").to_rgba8();
        assert_eq!(img.width(), 962);
        assert_eq!(img.height(), 880);

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
        dummy
            .save_with_format(&src_img_path, image::ImageFormat::Png)
            .expect("save dummy hex");

        let generated = honeycomb_thumb_for_media(&src_img_path, true, Some([255, 100, 50]), 1);
        assert!(generated.is_some(), "Honeycomb thumbnail should be generated");
        let path = generated.unwrap();
        assert!(path.to_string_lossy().ends_with("_act_ff6432.png"));

        let img = image::open(&path).expect("open generated hex thumb").to_rgba8();
        assert_eq!(img.width(), 260);
        assert_eq!(img.height(), 300);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(img.get_pixel(130, 150)[3], 255);

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
        cache.insert("d".into(), 4);
        assert_eq!(cache.get(&"b".into()), None);
        assert_eq!(cache.get(&"a".into()), Some(1));
        assert_eq!(cache.get(&"c".into()), Some(3));
        assert_eq!(cache.get(&"d".into()), Some(4));
        cache.clear();
        assert_eq!(cache.get(&"a".into()), None);
        assert_eq!(cache.get(&"c".into()), None);
        assert_eq!(cache.get(&"d".into()), None);
    }

    #[test]
    fn test_weighted_texture_cache_evicts_by_bytes() {
        let mut cache = TextureCache::new(10_000);
        let handle = cosmic::iced::widget::image::Handle::from_rgba(
            10,
            10,
            vec![0; 10 * 10 * 4],
        );
        cache.insert("a".into(), handle.clone(), 10, 10);
        cache.insert("b".into(), handle, 10, 10);
        assert!(cache.get("a").is_some());
        assert!(cache.get("b").is_some());
        assert!(cache.used_bytes() > 0);
    }

    #[test]
    fn test_poster_for_video_uses_high_resolution_master() {
        if std::process::Command::new("ffmpeg")
            .arg("-version")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| !status.success())
            .unwrap_or(true)
        {
            return;
        }

        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("aura_poster_test_{}", stamp));
        let _ = std::fs::create_dir_all(&test_dir);
        let video = test_dir.join("sample.mp4");
        let generated = std::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=320x180:rate=12",
                "-t",
                "2",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&video)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if !generated {
            let _ = std::fs::remove_dir_all(&test_dir);
            return;
        }

        let poster = poster_for_media(&video).expect("poster for test video");
        let dimensions = image::image_dimensions(&poster).expect("poster dimensions");
        assert_eq!(dimensions.0, 1280);
        assert_eq!(dimensions.1, 720);

        let _ = std::fs::remove_file(&poster);
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_trim_memory_non_panicking() {
        trim_memory();
    }
}

static THUMB_SEMAPHORE: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();

fn generate_thumb_sync(media_path: &Path) -> Option<PathBuf> {
    let thumb_path = thumb_path_for_video(media_path);
    if thumb_path.exists() {
        return Some(thumb_path);
    }
    if let Some(parent) = thumb_path.parent() {
        std::fs::create_dir_all(parent).ok()?;
    }
    let temp = atomic_temp_path(&thumb_path, "jpg");
    if is_image_path(media_path) {
        if let Ok(image) = image::open(media_path) {
            let thumbnail = image.thumbnail(480, 270);
            if thumbnail.save_with_format(&temp, image::ImageFormat::Jpeg).is_ok()
                && std::fs::rename(&temp, &thumb_path).is_ok()
            {
                return Some(thumb_path);
            }
        }
    }
    let status = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss")
        .arg("00:00:01")
        .arg("-i")
        .arg(media_path)
        .arg("-an")
        .arg("-sn")
        .arg("-vframes")
        .arg("1")
        .arg("-vf")
        .arg("scale=480:-2")
        .arg(&temp)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?;
    if status.success() && temp.exists() && std::fs::rename(&temp, &thumb_path).is_ok() {
        Some(thumb_path)
    } else {
        let _ = std::fs::remove_file(&temp);
        None
    }
}

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

    let src = media_path.to_path_buf();
    tokio::task::spawn_blocking(move || generate_thumb_sync(&src))
        .await
        .ok()
        .flatten()
}
