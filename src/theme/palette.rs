use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub struct RgbColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

pub struct PaletteResult {
    pub dominant_color: RgbColor,
    pub is_dark: bool,
}

fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let delta = max - min;

    let v = max;
    let s = if max == 0.0 { 0.0 } else { delta / max };

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let h = if h < 0.0 { h + 360.0 } else { h };
    (h, s, v)
}

pub fn extract_palette(thumb_path: &Path) -> Option<PaletteResult> {
    let img = image::open(thumb_path).ok()?.to_rgb8();
    // Sample a resized version for microsecond-fast processing
    let sample = image::imageops::resize(&img, 100, 100, image::imageops::FilterType::Nearest);

    let mut best_vibrancy = -1.0f32;
    let mut best_color = RgbColor { r: 1.0, g: 0.5, b: 0.0 };
    let mut total_lum = 0.0;
    let count = (sample.width() * sample.height()) as f64;

    for pixel in sample.pixels() {
        let pr = pixel[0] as f32 / 255.0;
        let pg = pixel[1] as f32 / 255.0;
        let pb = pixel[2] as f32 / 255.0;

        let (_, s, v) = rgb_to_hsv(pr, pg, pb);

        if v > 0.15 && v < 0.97 {
            let vibrancy = s * v;
            if vibrancy > best_vibrancy {
                best_vibrancy = vibrancy;
                best_color = RgbColor { r: pr, g: pg, b: pb };
            }
        }

        total_lum += 0.299 * (pixel[0] as f64) + 0.587 * (pixel[1] as f64) + 0.114 * (pixel[2] as f64);
    }

    let avg_lum = total_lum / count;
    let is_dark = avg_lum < 128.0;

    Some(PaletteResult {
        dominant_color: best_color,
        is_dark,
    })
}

fn darken(c: RgbColor, factor: f32) -> RgbColor {
    RgbColor {
        r: (c.r * factor).max(0.0),
        g: (c.g * factor).max(0.0),
        b: (c.b * factor).max(0.0),
    }
}

#[allow(dead_code)]
fn lighten(c: RgbColor, amount: f32) -> RgbColor {
    RgbColor {
        r: (c.r + amount).min(1.0),
        g: (c.g + amount).min(1.0),
        b: (c.b + amount).min(1.0),
    }
}

fn fmt_color(c: RgbColor, a: f32) -> String {
    format!(
        "(\n        red: {:.7},\n        green: {:.7},\n        blue: {:.7},\n        alpha: {:.1},\n    )",
        c.r, c.g, c.b, a
    )
}

pub fn apply_cosmic_theme(thumb_path: &Path, auto_dark: bool) -> bool {
    let Some(palette) = extract_palette(thumb_path) else {
        return false;
    };

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let cosmic_config = PathBuf::from(home).join(".config").join("cosmic");

    let dark_dir = cosmic_config.join("com.system76.CosmicTheme.Dark").join("v1");
    let dark_b_dir = cosmic_config.join("com.system76.CosmicTheme.Dark.Builder").join("v1");
    let light_dir = cosmic_config.join("com.system76.CosmicTheme.Light").join("v1");
    let light_b_dir = cosmic_config.join("com.system76.CosmicTheme.Light.Builder").join("v1");
    let mode_dir = cosmic_config.join("com.system76.CosmicTheme.Mode").join("v1");

    let col = palette.dominant_color;
    let dr = darken(col, 0.85);
    let pr = darken(col, 0.55);

    let accent_content = format!(
        "Some((\n    base: {},\n    hover: {},\n    pressed: {},\n    selected: {},\n    selected_text: {},\n    focus: {},\n    divider: {},\n    on: {},\n    disabled: {},\n    on_disabled: {},\n    border: {},\n    disabled_border: {},\n))",
        fmt_color(col, 1.0),
        fmt_color(dr, 1.0),
        fmt_color(pr, 1.0),
        fmt_color(dr, 1.0),
        fmt_color(col, 1.0),
        fmt_color(col, 1.0),
        fmt_color(RgbColor { r: 0.0, g: 0.0, b: 0.0 }, 1.0),
        fmt_color(RgbColor { r: 0.0, g: 0.0, b: 0.0 }, 1.0),
        fmt_color(col, 1.0),
        fmt_color(pr, 1.0),
        fmt_color(col, 1.0),
        fmt_color(col, 0.5),
    );

    let builder_accent = format!(
        "Some((\n    red: {:.7},\n    green: {:.7},\n    blue: {:.7},\n))",
        col.r, col.g, col.b
    );

    for (dir, b_dir, is_dark_variant) in [(&dark_dir, &dark_b_dir, true), (&light_dir, &light_b_dir, false)] {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::create_dir_all(b_dir);
        let _ = std::fs::write(dir.join("accent"), &accent_content);
        let _ = std::fs::write(b_dir.join("accent"), &builder_accent);
        let _ = std::fs::write(dir.join("is_dark"), if is_dark_variant { "true" } else { "false" });
    }

    if auto_dark {
        let _ = std::fs::create_dir_all(&mode_dir);
        let _ = std::fs::write(mode_dir.join("is_dark"), if palette.is_dark { "true" } else { "false" });
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_hsv() {
        // Red
        let (h, s, v) = rgb_to_hsv(1.0, 0.0, 0.0);
        assert!((h - 0.0).abs() < 0.01);
        assert!((s - 1.0).abs() < 0.01);
        assert!((v - 1.0).abs() < 0.01);

        // Green
        let (h, s, v) = rgb_to_hsv(0.0, 1.0, 0.0);
        assert!((h - 120.0).abs() < 0.01);
        assert!((s - 1.0).abs() < 0.01);
        assert!((v - 1.0).abs() < 0.01);

        // Blue
        let (h, s, v) = rgb_to_hsv(0.0, 0.0, 1.0);
        assert!((h - 240.0).abs() < 0.01);
        assert!((s - 1.0).abs() < 0.01);
        assert!((v - 1.0).abs() < 0.01);

        // Black
        let (h, s, v) = rgb_to_hsv(0.0, 0.0, 0.0);
        assert_eq!(h, 0.0);
        assert_eq!(s, 0.0);
        assert_eq!(v, 0.0);
    }

    #[test]
    fn test_darken_and_lighten() {
        let col = RgbColor { r: 0.8, g: 0.5, b: 0.2 };
        let dark = darken(col, 0.5);
        assert!((dark.r - 0.4).abs() < 0.01);
        assert!((dark.g - 0.25).abs() < 0.01);
        assert!((dark.b - 0.1).abs() < 0.01);

        let light = lighten(col, 0.3);
        assert!((light.r - 1.0).abs() < 0.01); // clamped to 1.0
        assert!((light.g - 0.8).abs() < 0.01);
        assert!((light.b - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_fmt_color() {
        let col = RgbColor { r: 1.0, g: 0.5, b: 0.0 };
        let formatted = fmt_color(col, 1.0);
        assert!(formatted.contains("red: 1.0000000"));
        assert!(formatted.contains("green: 0.5000000"));
        assert!(formatted.contains("blue: 0.0000000"));
        assert!(formatted.contains("alpha: 1.0"));
    }
}
