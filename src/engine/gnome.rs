// GNOME backend.
//
// Mutter does not implement wlr-layer-shell, so mpvpaper cannot draw a background
// layer on GNOME Wayland. On GNOME, Aura hands the wallpaper to the desktop instead:
// images go to the native org.gnome.desktop.background settings, and videos go to
// the Hanabi extension (https://github.com/jeffshee/gnome-ext-hanabi), which plays
// them from inside gnome-shell.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const HANABI_UUID: &str = "hanabi-extension@jeffshee.github.io";
const HANABI_SCHEMA: &str = "io.github.jeffshee.hanabi-extension";
const BACKGROUND_SCHEMA: &str = "org.gnome.desktop.background";
const RENDERER_DEST: &str = "io.github.jeffshee.HanabiRenderer";
const RENDERER_PATH: &str = "/io/github/jeffshee/HanabiRenderer";

/// True when Aura runs on a GNOME session. `AURA_BACKEND=gnome|mpvpaper` overrides detection.
pub fn is_active() -> bool {
    static ACTIVE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    // Unit tests must never touch the real desktop session.
    if cfg!(test) {
        return false;
    }
    *ACTIVE.get_or_init(|| {
        if let Ok(backend) = std::env::var("AURA_BACKEND") {
            return backend.eq_ignore_ascii_case("gnome");
        }
        std::env::var("XDG_CURRENT_DESKTOP")
            .map(|d| desktop_is_gnome(&d))
            .unwrap_or(false)
    })
}

fn desktop_is_gnome(xdg_current_desktop: &str) -> bool {
    xdg_current_desktop
        .split(':')
        .any(|part| part.eq_ignore_ascii_case("gnome"))
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

/// XDG data directories in lookup order: `$XDG_DATA_HOME`, then every `$XDG_DATA_DIRS` entry.
fn data_dirs() -> Vec<PathBuf> {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".local/share"));
    let data_dirs = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    std::iter::once(data_home)
        .chain(data_dirs.split(':').filter(|d| !d.is_empty()).map(PathBuf::from))
        .collect()
}

fn hanabi_extension_dirs() -> Vec<PathBuf> {
    data_dirs()
        .into_iter()
        .map(|d| d.join("gnome-shell/extensions").join(HANABI_UUID))
        .collect()
}

pub fn hanabi_installed() -> bool {
    hanabi_extension_dirs().iter().any(|d| d.join("metadata.json").exists())
}

/// Directory holding the compiled Hanabi schema when it is not in the system schema path.
fn hanabi_schema_dir() -> Option<PathBuf> {
    let file = format!("{}.gschema.xml", HANABI_SCHEMA);
    hanabi_extension_dirs()
        .into_iter()
        .map(|d| d.join("schemas"))
        .chain(data_dirs().into_iter().map(|d| d.join("glib-2.0/schemas")))
        .find(|d| d.join(&file).exists() && d.join("gschemas.compiled").exists())
}

/// Quotes a string as a GVariant text literal so gsettings never misparses it.
fn gvariant_str(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
}

/// Builds a file:// URI, percent-encoding everything except unreserved characters and '/'.
fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for b in path.to_string_lossy().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => uri.push(b as char),
            _ => uri.push_str(&format!("%{:02X}", b)),
        }
    }
    uri
}

fn run(cmd: &mut Command) -> Result<(), String> {
    let out = cmd
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("{:?}: {}", cmd.get_program(), e))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn gsettings_set(schema_dir: Option<&Path>, schema: &str, key: &str, value: &str) -> Result<(), String> {
    let mut cmd = Command::new("gsettings");
    if let Some(dir) = schema_dir {
        cmd.arg("--schemadir").arg(dir);
    }
    cmd.args(["set", schema, key, value]);
    run(&mut cmd)
}

fn set_hanabi_enabled(enabled: bool) -> Result<(), String> {
    run(Command::new("gnome-extensions")
        .arg(if enabled { "enable" } else { "disable" })
        .arg(HANABI_UUID))
}

fn renderer_call(method: &str) {
    let _ = run(Command::new("gdbus").args([
        "call",
        "--session",
        // The UI thread waits on this call; never block it for the 25 s D-Bus default.
        "--timeout",
        "2",
        "--dest",
        RENDERER_DEST,
        "--object-path",
        RENDERER_PATH,
        "--method",
        &format!("{}.{}", RENDERER_DEST, method),
    ]));
}

/// Sets a static image as the GNOME background (light and dark) and stops any Hanabi video.
pub fn set_image(path: &Path, scaling: &str) -> Result<(), String> {
    let options = match scaling {
        "fill" => "zoom",
        "stretch" => "stretched",
        _ => "scaled",
    };
    let uri = gvariant_str(&file_uri(path));
    gsettings_set(None, BACKGROUND_SCHEMA, "picture-options", &gvariant_str(options))?;
    gsettings_set(None, BACKGROUND_SCHEMA, "picture-uri", &uri)?;
    gsettings_set(None, BACKGROUND_SCHEMA, "picture-uri-dark", &uri)?;
    // A Hanabi video left running would cover the new image.
    if hanabi_installed() {
        set_hanabi_enabled(false).map_err(|e| format!("No se pudo desactivar Hanabi: {}", e))?;
    }
    Ok(())
}

fn hanabi_content_fit(scaling: &str) -> &'static str {
    // Gtk.ContentFit: 0 FILL (stretch), 1 CONTAIN, 2 COVER
    match scaling {
        "fill" => "2",
        "stretch" => "0",
        _ => "1",
    }
}

/// Hands a video to Hanabi and makes sure the extension is enabled and playing.
pub fn set_video(path: &Path, scaling: &str, mute: bool, volume: u8) -> Result<(), String> {
    if !hanabi_installed() {
        return Err(
            "GNOME no permite fondos de vídeo con mpvpaper. Instala la extensión Hanabi (gnome-ext-hanabi) para usar vídeos."
                .to_string(),
        );
    }
    let dir = hanabi_schema_dir();
    let dir = dir.as_deref();
    gsettings_set(dir, HANABI_SCHEMA, "video-path", &gvariant_str(&path.to_string_lossy()))?;
    gsettings_set(dir, HANABI_SCHEMA, "mute", if mute { "true" } else { "false" })?;
    gsettings_set(dir, HANABI_SCHEMA, "volume", &volume.min(100).to_string())?;
    gsettings_set(dir, HANABI_SCHEMA, "content-fit", hanabi_content_fit(scaling))?;
    set_hanabi_enabled(true).map_err(|e| {
        format!(
            "Hanabi está instalado pero GNOME no lo ha cargado ({}). Cierra sesión y vuelve a entrar.",
            e
        )
    })?;
    renderer_call("setPlay");
    Ok(())
}

pub fn set_mute(mute: bool) -> bool {
    gsettings_set(hanabi_schema_dir().as_deref(), HANABI_SCHEMA, "mute", if mute { "true" } else { "false" }).is_ok()
}

pub fn set_volume(volume: u8) -> bool {
    gsettings_set(hanabi_schema_dir().as_deref(), HANABI_SCHEMA, "volume", &volume.min(100).to_string()).is_ok()
}

pub fn set_scaling(scaling: &str) -> bool {
    let options = match scaling {
        "fill" => "zoom",
        "stretch" => "stretched",
        _ => "scaled",
    };
    let image_ok = gsettings_set(None, BACKGROUND_SCHEMA, "picture-options", &gvariant_str(options)).is_ok();
    let video_ok = hanabi_installed()
        && gsettings_set(hanabi_schema_dir().as_deref(), HANABI_SCHEMA, "content-fit", hanabi_content_fit(scaling)).is_ok();
    image_ok || video_ok
}

pub fn pause() {
    renderer_call("setPause");
}

pub fn play() {
    renderer_call("setPlay");
}

/// Stops the video wallpaper. The GNOME image background stays as the fallback.
pub fn stop() {
    if hanabi_installed() {
        if let Err(e) = set_hanabi_enabled(false) {
            eprintln!("[Aura Engine] No se pudo desactivar Hanabi: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_is_gnome() {
        assert!(desktop_is_gnome("GNOME"));
        assert!(desktop_is_gnome("ubuntu:GNOME"));
        assert!(!desktop_is_gnome("Hyprland"));
        assert!(!desktop_is_gnome("COSMIC"));
        assert!(!desktop_is_gnome(""));
    }

    #[test]
    fn test_file_uri_encoding() {
        assert_eq!(file_uri(Path::new("/a/b c.png")), "file:///a/b%20c.png");
        assert_eq!(file_uri(Path::new("/x/it's.png")), "file:///x/it%27s.png");
        assert_eq!(file_uri(Path::new("/x/ñ.png")), "file:///x/%C3%B1.png");
    }

    #[test]
    fn test_gvariant_str_escaping() {
        assert_eq!(gvariant_str("plain"), "'plain'");
        assert_eq!(gvariant_str("it's"), r"'it\'s'");
        assert_eq!(gvariant_str(r"a\b"), r"'a\\b'");
    }

    #[test]
    fn test_hanabi_content_fit() {
        assert_eq!(hanabi_content_fit("fill"), "2");
        assert_eq!(hanabi_content_fit("stretch"), "0");
        assert_eq!(hanabi_content_fit("fit"), "1");
    }
}
