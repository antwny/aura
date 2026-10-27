//! Desktop and wallpaper-backend detection.
//!
//! Aura used to treat every non-GNOME session as COSMIC. That is wrong on
//! Plasma, Hyprland, Sway, Niri, XFCE and every other compositor, and it caused
//! videos to be launched through mpvpaper on compositors where that surface is
//! hidden below the desktop shell (notably KDE Plasma). Keep the detection in
//! one place so the settings UI, the tray, the CLI and the engine agree.

use std::sync::OnceLock;

use crate::config::BackendPreference;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopEnvironment {
    Cosmic,
    Gnome,
    Kde,
    Hyprland,
    Sway,
    Niri,
    Xfce,
    Cinnamon,
    Mate,
    Lxqt,
    Budgie,
    Deepin,
    Unity,
    Other,
    Unknown,
}

impl Default for DesktopEnvironment {
    fn default() -> Self {
        Self::Unknown
    }
}

impl DesktopEnvironment {
    pub fn detect() -> Self {
        static DETECTED: OnceLock<DesktopEnvironment> = OnceLock::new();
        *DETECTED.get_or_init(|| {
            if let Ok(override_name) = std::env::var("AURA_DESKTOP") {
                if let Some(desktop) = Self::from_token(&override_name) {
                    return desktop;
                }
            }
            let mut raw = String::new();
            for key in [
                "XDG_CURRENT_DESKTOP",
                "XDG_SESSION_DESKTOP",
                "DESKTOP_SESSION",
                "GDMSESSION",
            ] {
                if let Ok(value) = std::env::var(key) {
                    raw.push(':');
                    raw.push_str(&value);
                }
            }
            Self::from_token(&raw).unwrap_or(Self::Unknown)
        })
    }

    /// Parses an XDG-style colon-separated desktop list. Exposed for tests.
    pub fn from_token(raw: &str) -> Option<Self> {
        let tokens: Vec<String> = raw
            .split([':', ';', ',', ' '])
            .filter(|token| !token.is_empty())
            .map(|token| token.to_ascii_lowercase())
            .collect();
        let has = |needle: &str| tokens.iter().any(|token| token.contains(needle));

        if has("gnome") {
            Some(Self::Gnome)
        } else if has("kde") || has("plasma") {
            Some(Self::Kde)
        } else if has("cosmic") {
            Some(Self::Cosmic)
        } else if has("hyprland") || has("hypr") {
            Some(Self::Hyprland)
        } else if has("sway") || has("wlroots") {
            Some(Self::Sway)
        } else if has("niri") {
            Some(Self::Niri)
        } else if has("xfce") {
            Some(Self::Xfce)
        } else if has("cinnamon") {
            Some(Self::Cinnamon)
        } else if has("mate") {
            Some(Self::Mate)
        } else if has("lxqt") {
            Some(Self::Lxqt)
        } else if has("budgie") {
            Some(Self::Budgie)
        } else if has("deepin") {
            Some(Self::Deepin)
        } else if has("unity") {
            Some(Self::Unity)
        } else if tokens.is_empty() {
            None
        } else {
            Some(Self::Other)
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Cosmic => "COSMIC Desktop",
            Self::Gnome => "GNOME",
            Self::Kde => "KDE Plasma",
            Self::Hyprland => "Hyprland",
            Self::Sway => "Sway",
            Self::Niri => "Niri",
            Self::Xfce => "Xfce",
            Self::Cinnamon => "Cinnamon",
            Self::Mate => "MATE",
            Self::Lxqt => "LXQt",
            Self::Budgie => "Budgie",
            Self::Deepin => "Deepin",
            Self::Unity => "Unity",
            Self::Other => "Other desktop",
            Self::Unknown => "Unknown session",
        }
    }

    pub fn is_wayland(self) -> bool {
        std::env::var("XDG_SESSION_TYPE")
            .map(|value| value.eq_ignore_ascii_case("wayland"))
            .unwrap_or_else(|_| std::env::var_os("WAYLAND_DISPLAY").is_some())
    }

    pub fn pretty_name(self) -> String {
        if self.is_wayland() {
            format!("{} (Wayland)", self.label())
        } else {
            self.label().to_string()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Mpvpaper,
    Gnome,
    Kde,
    Static,
}

impl Backend {
    pub fn detect(preference: BackendPreference) -> Self {
        if let Ok(env_backend) = std::env::var("AURA_BACKEND") {
            if let Some(backend) = Self::from_override(&env_backend) {
                return backend;
            }
        }
        match preference {
            BackendPreference::Mpvpaper => Self::Mpvpaper,
            BackendPreference::Gnome => Self::Gnome,
            BackendPreference::Kde => Self::Kde,
            BackendPreference::Static => Self::Static,
            BackendPreference::Auto => match DesktopEnvironment::detect() {
                DesktopEnvironment::Gnome => Self::Gnome,
                DesktopEnvironment::Kde => Self::Kde,
                _ => Self::Mpvpaper,
            },
        }
    }

    pub fn from_override(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "gnome" => Some(Self::Gnome),
            "kde" | "plasma" => Some(Self::Kde),
            "mpvpaper" | "layer-shell" | "layer_shell" | "wayland" => Some(Self::Mpvpaper),
            "static" | "image" => Some(Self::Static),
            "auto" => None,
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Mpvpaper => "mpvpaper (Wayland Layer Shell)",
            Self::Gnome => "GNOME Native (GSettings + Hanabi)",
            Self::Kde => "KDE Plasma Native (D-Bus)",
            Self::Static => "Static desktop wallpaper",
        }
    }

    pub fn uses_desktop_shell(self) -> bool {
        matches!(self, Self::Gnome | Self::Kde)
    }

    pub fn uses_desktop_shell_on(self, desktop: DesktopEnvironment) -> bool {
        self.uses_desktop_shell()
            || (self == Self::Static
                && matches!(desktop, DesktopEnvironment::Gnome | DesktopEnvironment::Kde))
    }
}

/// Optional second-line diagnostic used by Settings. The active engine is
/// already shown in its own row, so this only returns backend-specific extras
/// (plugin/tool status or an environment override) and is hidden when there is
/// nothing useful to add.
pub fn backend_diagnostic(preference: BackendPreference) -> Option<String> {
    let backend = Backend::detect(preference);
    let env_override = std::env::var("AURA_BACKEND")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("AURA_BACKEND={}", value));
    let details = match backend {
        Backend::Kde => {
            if !crate::engine::kde::dbus_available() {
                Some("qdbus missing: install Qt D-Bus tools to apply wallpapers".to_string())
            } else if crate::engine::kde::plugin_installed() {
                Some("Smart Video Wallpaper Reborn detected".to_string())
            } else {
                Some("plugin missing: videos fall back to their poster image".to_string())
            }
        }
        Backend::Gnome => {
            if crate::engine::gnome::hanabi_installed() {
                Some("Hanabi detected".to_string())
            } else {
                Some("Hanabi missing: videos unavailable".to_string())
            }
        }
        _ => None,
    };
    match (details, env_override) {
        (Some(details), Some(env_override)) => Some(format!("{} · {}", details, env_override)),
        (Some(details), None) => Some(details),
        (None, Some(env_override)) => Some(env_override),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_desktop_tokens() {
        assert_eq!(
            DesktopEnvironment::from_token("ubuntu:GNOME"),
            Some(DesktopEnvironment::Gnome)
        );
        assert_eq!(
            DesktopEnvironment::from_token("KDE"),
            Some(DesktopEnvironment::Kde)
        );
        assert_eq!(
            DesktopEnvironment::from_token("plasma"),
            Some(DesktopEnvironment::Kde)
        );
        assert_eq!(
            DesktopEnvironment::from_token("COSMIC"),
            Some(DesktopEnvironment::Cosmic)
        );
        assert_eq!(
            DesktopEnvironment::from_token("Hyprland"),
            Some(DesktopEnvironment::Hyprland)
        );
        assert_eq!(
            DesktopEnvironment::from_token("sway"),
            Some(DesktopEnvironment::Sway)
        );
        assert_eq!(
            DesktopEnvironment::from_token("niri"),
            Some(DesktopEnvironment::Niri)
        );
        assert_eq!(
            DesktopEnvironment::from_token("XFCE"),
            Some(DesktopEnvironment::Xfce)
        );
        assert_eq!(
            DesktopEnvironment::from_token("i3"),
            Some(DesktopEnvironment::Other)
        );
        assert_eq!(DesktopEnvironment::from_token(""), None);
    }

    #[test]
    fn backend_override_aliases_work() {
        assert_eq!(Backend::from_override("gnome"), Some(Backend::Gnome));
        assert_eq!(Backend::from_override("plasma"), Some(Backend::Kde));
        assert_eq!(Backend::from_override("mpvpaper"), Some(Backend::Mpvpaper));
        assert_eq!(Backend::from_override("static"), Some(Backend::Static));
        assert_eq!(Backend::from_override("auto"), None);
        assert_eq!(Backend::from_override("nonsense"), None);
    }

    #[test]
    fn labels_are_not_cosmic_by_default() {
        assert_ne!(DesktopEnvironment::Kde.label(), "COSMIC Desktop");
        assert_ne!(DesktopEnvironment::Hyprland.label(), "COSMIC Desktop");
    }

    #[test]
    fn diagnostics_do_not_repeat_the_engine_label() {
        for preference in [
            BackendPreference::Auto,
            BackendPreference::Mpvpaper,
            BackendPreference::Static,
        ] {
            if let Some(diagnostic) = backend_diagnostic(preference) {
                assert!(!diagnostic.contains(Backend::Mpvpaper.label()));
                assert!(!diagnostic.contains(Backend::Static.label()));
            }
        }
    }
}
