use std::path::Path;

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

    // If no vibrant hue was found (monochrome, black & white, or grayscale image),
    // use an elegant neutral slate tone instead of default neon orange
    if best_vibrancy < 0.06 {
        best_color = if is_dark {
            RgbColor { r: 0.45, g: 0.55, b: 0.65 } // Slate blue-gray for dark wallpapers
        } else {
            RgbColor { r: 0.35, g: 0.45, b: 0.55 } // Deep slate gray for light wallpapers
        };
    }

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

fn atomic_write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let pid = std::process::id();
    let tmp = path.with_extension(format!("tmp.{}", pid));
    if std::fs::write(&tmp, content).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

pub fn apply_cosmic_mode(is_dark: bool) {
    let cosmic_config = crate::config::config_home().join("cosmic");
    let mode_dir = cosmic_config.join("com.system76.CosmicTheme.Mode").join("v1");
    atomic_write(&mode_dir.join("is_dark"), if is_dark { "true\n" } else { "false\n" });
}

/// Extract only the accent color used by Aura's own libcosmic theme.
pub fn extract_accent(thumb_path: &Path) -> Option<RgbColor> {
    extract_palette(thumb_path).map(|palette| palette.dominant_color)
}

/// Build an app-local libcosmic theme with the wallpaper accent. This never
/// modifies COSMIC/GNOME/KDE settings; it only styles Aura itself.
pub fn app_theme_with_accent(accent: RgbColor) -> cosmic::Theme {
    use cosmic::cosmic_config::CosmicConfigEntry;
    use cosmic::cosmic_theme::palette::Srgb;
    use cosmic::cosmic_theme::ThemeBuilder;

    let base = cosmic::theme::system_preference();
    let base_theme = base.cosmic();
    let accent = Srgb::new(
        accent.r.clamp(0.0, 1.0),
        accent.g.clamp(0.0, 1.0),
        accent.b.clamp(0.0, 1.0),
    );

    // libcosmic's `Theme::with_accent` uses `ThemeBuilder::get_entry(&cfg).ok()`
    // internally. If a COSMIC version left any builder key unset, `get_entry`
    // still returns a usable partial builder together with the missing-key
    // error, but `with_accent` throws that builder away and falls back to the
    // default theme. That silently resets frosted glass and alpha settings.
    // Load the builder ourselves and keep the partial result.
    let config = if base_theme.is_dark {
        ThemeBuilder::dark_config()
    } else {
        ThemeBuilder::light_config()
    };
    let mut builder = config
        .ok()
        .map(|cfg| ThemeBuilder::get_entry(&cfg).unwrap_or_else(|(_, builder)| builder))
        .unwrap_or_else(|| {
            if base_theme.is_dark {
                ThemeBuilder::dark()
            } else {
                ThemeBuilder::light()
            }
        });

    // The builder config can lag behind the active theme (COSMIC keeps the
    // generated theme and its builder in separate config trees). Mirror the
    // live values before building so the generated transparent surfaces use
    // the same alpha/frosted values as the rest of the desktop.
    builder.frosted = base_theme.frosted;
    builder.frosted_windows = base_theme.frosted_windows;
    builder.frosted_system_interface = base_theme.frosted_system_interface;
    builder.frosted_panel = base_theme.frosted_panel;
    builder.frosted_applets = base_theme.frosted_applets;
    builder.frosted_maximized_apps = base_theme.frosted_maximized_apps;
    builder.alpha_map = base_theme.alpha_map.clone();
    builder.spacing = base_theme.spacing.clone();
    builder.corner_radii = base_theme.corner_radii.clone();
    builder.gaps = base_theme.gaps;
    builder.active_hint = base_theme.active_hint;
    builder.window_hint = base_theme.window_hint;

    let mut theme = builder.accent(accent).build();

    // Keep the live frosted/alpha values authoritative even if the builder
    // had to fall back to defaults.
    theme.frosted = base_theme.frosted;
    theme.frosted_windows = base_theme.frosted_windows;
    theme.frosted_system_interface = base_theme.frosted_system_interface;
    theme.frosted_panel = base_theme.frosted_panel;
    theme.frosted_applets = base_theme.frosted_applets;
    theme.frosted_maximized_apps = base_theme.frosted_maximized_apps;
    theme.alpha_map = base_theme.alpha_map.clone();

    cosmic::Theme::custom(std::sync::Arc::new(theme))
}

pub fn apply_cosmic_theme(
    thumb_path: &Path,
    auto_theme: bool,
    theme_mode: crate::config::ThemeMode,
    write_system_accent: bool,
) -> bool {
    if !auto_theme && theme_mode == crate::config::ThemeMode::Manual {
        return false;
    }

    let palette_opt = extract_palette(thumb_path);

    // 1. Apply system appearance mode
    match theme_mode {
        crate::config::ThemeMode::Dark => apply_cosmic_mode(true),
        crate::config::ThemeMode::Light => apply_cosmic_mode(false),
        crate::config::ThemeMode::Auto => {
            if let Some(ref pal) = palette_opt {
                apply_cosmic_mode(pal.is_dark);
            }
        }
        crate::config::ThemeMode::Manual => {}
    }

    // 2. Apply dynamic accent color if auto_theme is enabled. GNOME and KDE
    // ignore COSMIC config, so only write it when running on COSMIC; Aura's
    // own UI gets the accent through `app_theme_with_accent` instead.
    if auto_theme {
        let Some(palette) = palette_opt else {
            return false;
        };
        if !write_system_accent
            || crate::engine::DesktopEnvironment::detect()
                != crate::engine::DesktopEnvironment::Cosmic
        {
            return true;
        }

        let cosmic_config = crate::config::config_home().join("cosmic");

        let col = palette.dominant_color;
        let r_u8 = (col.r * 255.0).round().clamp(0.0, 255.0) as u8;
        let g_u8 = (col.g * 255.0).round().clamp(0.0, 255.0) as u8;
        let b_u8 = (col.b * 255.0).round().clamp(0.0, 255.0) as u8;

        let dr = darken(col, 0.85);
        let dr_r = (dr.r * 255.0).round().clamp(0.0, 255.0) as u8;
        let dr_g = (dr.g * 255.0).round().clamp(0.0, 255.0) as u8;
        let dr_b = (dr.b * 255.0).round().clamp(0.0, 255.0) as u8;

        let pr = darken(col, 0.55);
        let pr_r = (pr.r * 255.0).round().clamp(0.0, 255.0) as u8;
        let pr_g = (pr.g * 255.0).round().clamp(0.0, 255.0) as u8;
        let pr_b = (pr.b * 255.0).round().clamp(0.0, 255.0) as u8;

        let base_hex = format!("#{:02X}{:02X}{:02X}FF", r_u8, g_u8, b_u8);
        let hover_hex = format!("#{:02X}{:02X}{:02X}FF", dr_r, dr_g, dr_b);
        let pressed_hex = format!("#{:02X}{:02X}{:02X}FF", pr_r, pr_g, pr_b);
        let disabled_border_hex = format!("#{:02X}{:02X}{:02X}80", r_u8, g_u8, b_u8);

        // WCAG relative luminance for button foreground text contrast
        let lum = 0.299 * (r_u8 as f64 / 255.0) + 0.587 * (g_u8 as f64 / 255.0) + 0.114 * (b_u8 as f64 / 255.0);
        let (on_hex, on_disabled_hex) = if lum > 0.5 {
            ("#000000FF", "#333333FF")
        } else {
            ("#FFFFFFFF", "#AAAAAAFF")
        };

        let builder_accent_v2 = format!("Some(\"{}\")\n", base_hex);

        for (variant, divider_hex) in [("Dark", "#000000FF"), ("Light", "#FFFFFFFF")] {
            let accent_content_v2 = format!(
                "(\n    base: \"{}\",\n    hover: \"{}\",\n    pressed: \"{}\",\n    selected: \"{}\",\n    selected_text: \"{}\",\n    focus: \"{}\",\n    divider: \"{}\",\n    on: \"{}\",\n    disabled: \"{}\",\n    on_disabled: \"{}\",\n    border: \"{}\",\n    disabled_border: \"{}\",\n)\n",
                base_hex, hover_hex, pressed_hex, hover_hex, base_hex, base_hex, divider_hex, on_hex, base_hex, on_disabled_hex, base_hex, disabled_border_hex
            );

            // 1. COSMIC v2 paths (Active system standard in Pop!_OS COSMIC)
            let b_v2 = cosmic_config.join(format!("com.system76.CosmicTheme.{}.Builder", variant)).join("v2");
            let t_v2 = cosmic_config.join(format!("com.system76.CosmicTheme.{}", variant)).join("v2");
            atomic_write(&b_v2.join("accent"), &builder_accent_v2);
            atomic_write(&t_v2.join("accent"), &accent_content_v2);
            atomic_write(&t_v2.join("accent_button"), &accent_content_v2);

            // 2. COSMIC v1 paths (Backwards compatibility)
            let b_v1 = cosmic_config.join(format!("com.system76.CosmicTheme.{}.Builder", variant)).join("v1");
            let t_v1 = cosmic_config.join(format!("com.system76.CosmicTheme.{}", variant)).join("v1");
            let builder_accent_v1 = format!(
                "Some((\n    red: {:.7},\n    green: {:.7},\n    blue: {:.7},\n))\n",
                col.r, col.g, col.b
            );
            let accent_content_v1 = format!(
                "Some((\n    base: {},\n    hover: {},\n    pressed: {},\n    selected: {},\n    selected_text: {},\n    focus: {},\n    divider: {},\n    on: {},\n    disabled: {},\n    on_disabled: {},\n    border: {},\n    disabled_border: {},\n))\n",
                fmt_color(col, 1.0),
                fmt_color(dr, 1.0),
                fmt_color(pr, 1.0),
                fmt_color(dr, 1.0),
                fmt_color(col, 1.0),
                fmt_color(col, 1.0),
                fmt_color(RgbColor { r: 0.0, g: 0.0, b: 0.0 }, 1.0),
                fmt_color(if lum > 0.5 { RgbColor { r: 0.0, g: 0.0, b: 0.0 } } else { RgbColor { r: 1.0, g: 1.0, b: 1.0 } }, 1.0),
                fmt_color(col, 1.0),
                fmt_color(pr, 1.0),
                fmt_color(col, 1.0),
                fmt_color(col, 0.5),
            );
            atomic_write(&b_v1.join("accent"), &builder_accent_v1);
            atomic_write(&t_v1.join("accent"), &accent_content_v1);
        }
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
    fn test_extract_accent_from_image() {
        let temp_dir = std::env::temp_dir().join(format!("aura_test_accent_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let path = temp_dir.join("accent.png");
        let image = image::RgbImage::from_pixel(16, 16, image::Rgb([220, 30, 45]));
        image.save(&path).unwrap();

        let accent = extract_accent(&path).expect("accent from solid image");
        assert!(accent.r > 0.8);
        assert!(accent.g < 0.2);
        assert!(accent.b < 0.25);

        let _ = std::fs::remove_dir_all(temp_dir);
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

    #[test]
    fn test_apply_disabled() {
        // When auto_theme is false and theme_mode is Manual, should return false immediately
        assert!(!apply_cosmic_theme(
            Path::new("/nonexistent"),
            false,
            crate::config::ThemeMode::Manual,
            true,
        ));
    }

    #[test]
    fn test_apply_theme_modes_without_accent() {
        // ThemeMode::Dark and Light return true even if auto_theme is false (mode is applied)
        assert!(apply_cosmic_theme(
            Path::new("/nonexistent"),
            false,
            crate::config::ThemeMode::Dark,
            true,
        ));
        assert!(apply_cosmic_theme(
            Path::new("/nonexistent"),
            false,
            crate::config::ThemeMode::Light,
            true,
        ));
        // Non-COSMIC desktops still apply the mode locally but skip the
        // system accent files.
        assert!(apply_cosmic_theme(
            Path::new("/nonexistent"),
            false,
            crate::config::ThemeMode::Dark,
            false,
        ));
    }

    #[test]
    fn test_apply_theme_skips_system_accent_when_disabled() {
        let temp_dir = std::env::temp_dir().join(format!("aura_test_no_accent_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let path = temp_dir.join("accent.png");
        image::RgbImage::from_pixel(8, 8, image::Rgb([40, 80, 200]))
            .save(&path)
            .unwrap();

        let accent_path = crate::config::config_home()
            .join("cosmic")
            .join("com.system76.CosmicTheme.Dark")
            .join("v2")
            .join("accent");
        let _ = std::fs::remove_file(&accent_path);

        assert!(apply_cosmic_theme(
            &path,
            true,
            crate::config::ThemeMode::Dark,
            false,
        ));
        assert!(!accent_path.exists());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_atomic_write() {
        let temp_dir = std::env::temp_dir().join(format!("aura_test_theme_{}", std::process::id()));
        let target = temp_dir.join("test_file");
        atomic_write(&target, "hello world");
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "hello world");
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
