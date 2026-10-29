//! User-facing CLI strings and auto-pause mode labels.
//!
//! Keeping the text here (instead of hardcoding Spanish in `cli.rs`) makes the
//! terminal follow the same language as the GUI and the saved configuration.

use crate::config::AutoPauseMode;
use crate::i18n::Language;

/// Fixed labels used by `aura status`. Dynamic values are formatted by the CLI
/// so the alignment stays stable across languages.
pub struct CliLabels {
    pub status_title: &'static str,
    pub desktop: &'static str,
    pub backend: &'static str,
    pub engine: &'static str,
    pub engine_ready: &'static str,
    pub engine_missing_binary: &'static str,
    pub engine_missing_library: &'static str,
    pub engine_error: &'static str,
    pub install_hint: &'static str,
    pub run_state: &'static str,
    pub playing: &'static str,
    pub paused: &'static str,
    pub stopped: &'static str,
    pub wallpaper: &'static str,
    pub none: &'static str,
    pub displays: &'static str,
    pub scaling: &'static str,
    pub rotation: &'static str,
    pub auto_theme: &'static str,
    pub appearance_mode: &'static str,
    pub muted: &'static str,
    pub volume: &'static str,
    pub gpu_accel: &'static str,
    pub gpu_pref: &'static str,
    pub autostart: &'static str,
    pub auto_pause: &'static str,
    pub enabled: &'static str,
    pub disabled: &'static str,
    pub yes: &'static str,
    pub no: &'static str,
    pub dbus: &'static str,
    pub available: &'static str,
    pub missing: &'static str,
    pub plugin: &'static str,
    pub plugin_detected: &'static str,
    pub plugin_missing: &'static str,
    pub pool_all: &'static str,
    pub pool_favorites: &'static str,
    pub media_all: &'static str,
    pub media_live: &'static str,
    pub media_static: &'static str,
    pub order_random: &'static str,
    pub order_sequential: &'static str,
    pub theme_auto: &'static str,
    pub theme_dark: &'static str,
    pub theme_light: &'static str,
    pub theme_manual: &'static str,
    pub auto_pause_full: &'static str,
    pub auto_pause_maximized: &'static str,
    pub auto_pause_partial: &'static str,
    pub auto_pause_any_window: &'static str,
}

impl Language {
    pub fn cli_labels(self) -> CliLabels {
        match self {
            Language::Es => CliLabels {
                status_title: "🌌 Aura Live Wallpaper — Estado del Sistema",
                desktop: "Escritorio:",
                backend: "Backend:",
                engine: "Motor multimedia:",
                engine_ready: "Listo (mpvpaper + libmpv funcional)",
                engine_missing_binary: "⚠️  mpvpaper no encontrado",
                engine_missing_library: "⚠️  Falta librería multimedia (libmpv)",
                engine_error: "⚠️  Error de verificación",
                install_hint: "👉 En",
                run_state: "Estado de ejecución:",
                playing: "▶  En reproducción",
                paused: "⏸  Pausado (0% CPU/GPU)",
                stopped: "⏹  Detenido",
                wallpaper: "Fondo configurado:",
                none: "Ninguno",
                displays: "Pantallas configuradas:",
                scaling: "Escala",
                rotation: "Rotación de fondos:",
                auto_theme: "Auto-Tema:",
                appearance_mode: "Modo de apariencia:",
                muted: "Silenciado:",
                volume: "Volumen:",
                gpu_accel: "Aceleración GPU:",
                gpu_pref: "Preferencia GPU:",
                autostart: "Inicio automático:",
                auto_pause: "Auto-Pausa:",
                enabled: "Activado",
                disabled: "Desactivado",
                yes: "Sí",
                no: "No",
                dbus: "D-Bus Plasma:",
                available: "Disponible",
                missing: "⚠️  qdbus6/qdbus no encontrado",
                plugin: "Plugin KDE:",
                plugin_detected: "Smart Video Wallpaper Reborn detectado",
                plugin_missing: "⚠️  Smart Video Wallpaper Reborn no instalado (fallback estático)",
                pool_all: "Toda la biblioteca",
                pool_favorites: "Solo favoritos",
                media_all: "Animados + estáticos",
                media_live: "Solo animados",
                media_static: "Solo estáticos",
                order_random: "Aleatorio",
                order_sequential: "Secuencial",
                theme_auto: "Automático (dinámico según luminancia)",
                theme_dark: "Oscuro (Fijo y persistente)",
                theme_light: "Claro (Fijo y persistente)",
                theme_manual: "Sin alterar (Del sistema)",
                auto_pause_full: "Solo pantalla completa",
                auto_pause_maximized: "Ventanas maximizadas",
                auto_pause_partial: "Ventanas en mosaico (≥25%)",
                auto_pause_any_window: "Cualquier ventana",
            },
            Language::En => CliLabels {
                status_title: "🌌 Aura Live Wallpaper — System Status",
                desktop: "Desktop:",
                backend: "Backend:",
                engine: "Media engine:",
                engine_ready: "Ready (mpvpaper + working libmpv)",
                engine_missing_binary: "⚠️  mpvpaper not found",
                engine_missing_library: "⚠️  Missing media library (libmpv)",
                engine_error: "⚠️  Health check error",
                install_hint: "👉 On",
                run_state: "Run state:",
                playing: "▶  Playing",
                paused: "⏸  Paused (0% CPU/GPU)",
                stopped: "⏹  Stopped",
                wallpaper: "Configured wallpaper:",
                none: "None",
                displays: "Configured displays:",
                scaling: "Scale",
                rotation: "Wallpaper rotation:",
                auto_theme: "Auto theme:",
                appearance_mode: "Appearance mode:",
                muted: "Muted:",
                volume: "Volume:",
                gpu_accel: "GPU acceleration:",
                gpu_pref: "GPU preference:",
                autostart: "Autostart:",
                auto_pause: "Auto-pause:",
                enabled: "Enabled",
                disabled: "Disabled",
                yes: "Yes",
                no: "No",
                dbus: "Plasma D-Bus:",
                available: "Available",
                missing: "⚠️  qdbus6/qdbus not found",
                plugin: "KDE plugin:",
                plugin_detected: "Smart Video Wallpaper Reborn detected",
                plugin_missing: "⚠️  Smart Video Wallpaper Reborn not installed (static fallback)",
                pool_all: "Whole library",
                pool_favorites: "Favorites only",
                media_all: "Animated + static",
                media_live: "Animated only",
                media_static: "Static only",
                order_random: "Random",
                order_sequential: "Sequential",
                theme_auto: "Automatic (luminance-based)",
                theme_dark: "Dark (fixed and persistent)",
                theme_light: "Light (fixed and persistent)",
                theme_manual: "Unchanged (system)",
                auto_pause_full: "Fullscreen only",
                auto_pause_maximized: "Maximized windows",
                auto_pause_partial: "Tiled windows (≥25%)",
                auto_pause_any_window: "Any window",
            },
            Language::Fr => CliLabels {
                status_title: "🌌 Aura Live Wallpaper — État du système",
                desktop: "Bureau :",
                backend: "Backend :",
                engine: "Moteur multimédia :",
                engine_ready: "Prêt (mpvpaper + libmpv fonctionnel)",
                engine_missing_binary: "⚠️  mpvpaper introuvable",
                engine_missing_library: "⚠️  Bibliothèque multimédia manquante (libmpv)",
                engine_error: "⚠️  Erreur de vérification",
                install_hint: "👉 Sous",
                run_state: "État d’exécution :",
                playing: "▶  En lecture",
                paused: "⏸  En pause (0 % CPU/GPU)",
                stopped: "⏹  Arrêté",
                wallpaper: "Fond configuré :",
                none: "Aucun",
                displays: "Écrans configurés :",
                scaling: "Échelle",
                rotation: "Rotation des fonds :",
                auto_theme: "Thème auto :",
                appearance_mode: "Mode d’apparence :",
                muted: "Muet :",
                volume: "Volume :",
                gpu_accel: "Accélération GPU :",
                gpu_pref: "Préférence GPU :",
                autostart: "Démarrage automatique :",
                auto_pause: "Pause auto :",
                enabled: "Activé",
                disabled: "Désactivé",
                yes: "Oui",
                no: "Non",
                dbus: "D-Bus Plasma :",
                available: "Disponible",
                missing: "⚠️  qdbus6/qdbus introuvable",
                plugin: "Plugin KDE :",
                plugin_detected: "Smart Video Wallpaper Reborn détecté",
                plugin_missing: "⚠️  Smart Video Wallpaper Reborn non installé (repli statique)",
                pool_all: "Toute la bibliothèque",
                pool_favorites: "Favoris uniquement",
                media_all: "Animés + statiques",
                media_live: "Animés uniquement",
                media_static: "Statiques uniquement",
                order_random: "Aléatoire",
                order_sequential: "Séquentiel",
                theme_auto: "Automatique (selon la luminance)",
                theme_dark: "Sombre (fixe et persistant)",
                theme_light: "Clair (fixe et persistant)",
                theme_manual: "Inchangé (système)",
                auto_pause_full: "Plein écran uniquement",
                auto_pause_maximized: "Fenêtres maximisées",
                auto_pause_partial: "Fenêtres en mosaïque (≥25 %)",
                auto_pause_any_window: "Toute fenêtre",
            },
        }
    }

    /// Localized name for an auto-pause mode (also used by Settings).
    pub fn auto_pause_mode_name(self, mode: AutoPauseMode) -> &'static str {
        let labels = self.cli_labels();
        match mode {
            AutoPauseMode::Full => labels.auto_pause_full,
            AutoPauseMode::Maximized => labels.auto_pause_maximized,
            AutoPauseMode::Partial => labels.auto_pause_partial,
            AutoPauseMode::AnyWindow => labels.auto_pause_any_window,
        }
    }

    pub fn auto_pause_mode_choices(self) -> [(AutoPauseMode, &'static str); 4] {
        [
            (AutoPauseMode::Full, self.auto_pause_mode_name(AutoPauseMode::Full)),
            (
                AutoPauseMode::Maximized,
                self.auto_pause_mode_name(AutoPauseMode::Maximized),
            ),
            (
                AutoPauseMode::Partial,
                self.auto_pause_mode_name(AutoPauseMode::Partial),
            ),
            (
                AutoPauseMode::AnyWindow,
                self.auto_pause_mode_name(AutoPauseMode::AnyWindow),
            ),
        ]
    }

    /// Explanation of the currently selected sensitivity, shown under the
    /// mode buttons in Settings.
    pub fn auto_pause_mode_hint(self, mode: AutoPauseMode) -> &'static str {
        match (self, mode) {
            (Language::Es, AutoPauseMode::Full) => {
                "Solo se pausa cuando una ventana cubre toda la pantalla (pantalla completa o maximizada)."
            }
            (Language::Es, AutoPauseMode::Maximized) => {
                "Se pausa con ventanas maximizadas o a pantalla completa; las ventanas en mosaico no pausan."
            }
            (Language::Es, AutoPauseMode::Partial) => {
                "Se pausa cuando las ventanas cubren al menos el 25% del monitor. Recomendado en COSMIC."
            }
            (Language::Es, AutoPauseMode::AnyWindow) => {
                "Se pausa con cualquier ventana visible, incluso pequeñas o diálogos. Puede pausar más de lo esperado."
            }
            (Language::En, AutoPauseMode::Full) => {
                "Pauses only when a window covers the whole display (fullscreen or maximized)."
            }
            (Language::En, AutoPauseMode::Maximized) => {
                "Pauses for maximized or fullscreen windows; tiled windows do not pause it."
            }
            (Language::En, AutoPauseMode::Partial) => {
                "Pauses when windows cover at least 25% of the monitor. Recommended on COSMIC."
            }
            (Language::En, AutoPauseMode::AnyWindow) => {
                "Pauses with any visible window, including small windows or dialogs. It may pause more than expected."
            }
            (Language::Fr, AutoPauseMode::Full) => {
                "Met en pause uniquement lorsqu’une fenêtre couvre tout l’écran (plein écran ou maximisée)."
            }
            (Language::Fr, AutoPauseMode::Maximized) => {
                "Met en pause pour les fenêtres maximisées ou en plein écran ; les fenêtres en mosaïque ne la déclenchent pas."
            }
            (Language::Fr, AutoPauseMode::Partial) => {
                "Met en pause lorsque les fenêtres couvrent au moins 25 % de l’écran. Recommandé sous COSMIC."
            }
            (Language::Fr, AutoPauseMode::AnyWindow) => {
                "Met en pause dès qu’une fenêtre est visible, même petite ou une boîte de dialogue. La pause peut être plus fréquente que prévu."
            }
        }
    }

    pub fn settings_auto_pause_mode(self) -> &'static str {
        match self {
            Language::Es => "Sensibilidad de Auto-Pausa",
            Language::En => "Auto-pause sensitivity",
            Language::Fr => "Sensibilité de la pause auto",
        }
    }

    /// Value part of the rotation line in `aura status`.
    pub fn cli_rotation_line(
        self,
        enabled: bool,
        pool: &str,
        media: &str,
        random: bool,
        interval: u64,
    ) -> String {
        let labels = self.cli_labels();
        let state = if enabled {
            labels.enabled.to_lowercase()
        } else {
            labels.disabled.to_lowercase()
        };
        let order = if random {
            labels.order_random
        } else {
            labels.order_sequential
        };
        match self {
            Language::Es => format!(
                "{} ({}, {}, {}, cada {} min)",
                state, pool, media, order, interval
            ),
            Language::En => format!(
                "{} ({}, {}, {}, every {} min)",
                state, pool, media, order, interval
            ),
            Language::Fr => format!(
                "{} ({}, {}, {}, toutes les {} min)",
                state, pool, media, order, interval
            ),
        }
    }

    pub fn cli_help(self) -> &'static str {
        match self {
            Language::Es => r#"🌌 Aura — Gestor Nativo de Fondos Animados para escritorios Wayland

Uso:
  aura                  Abre la interfaz gráfica de Aura
  aura switcher         Abre el selector rápido de fondos (HUD)
  aura next             Cambia al siguiente fondo
  aura prev             Vuelve al fondo anterior
  aura stop             Detiene la reproducción de fondos
  aura toggle-pause     Pausa o reanuda (0% GPU al pausar)
  aura apply <archivo>  Aplica inmediatamente un vídeo o imagen
  aura status           Muestra el estado del fondo y las pantallas
  aura mode <modo>      Cambia el tema: dark, light, auto, manual
  aura check-update     Comprueba si hay una nueva versión
  aura update           Descarga e instala la última actualización
  aura --version, -v    Muestra la versión de Aura
  aura help             Muestra esta ayuda

Consejo: asigna 'aura next' a un atajo de teclado (p. ej. Super + W)."#,
            Language::En => r#"🌌 Aura — Native Live Wallpaper Manager for Wayland desktops

Usage:
  aura                  Open the Aura graphical interface
  aura switcher         Open the quick wallpaper switcher (HUD)
  aura next             Switch to the next wallpaper
  aura prev             Go back to the previous wallpaper
  aura stop             Stop wallpaper playback
  aura toggle-pause     Pause or resume (0% GPU while paused)
  aura apply <file>     Immediately apply a video or image
  aura status           Show wallpaper and display status
  aura mode <mode>      Change the theme: dark, light, auto, manual
  aura check-update     Check for a new release
  aura update           Download and install the latest update
  aura --version, -v    Show the Aura version
  aura help             Show this help

Tip: bind 'aura next' to a keyboard shortcut (for example Super + W)."#,
            Language::Fr => r#"🌌 Aura — Gestionnaire natif de fonds animés pour Wayland

Utilisation :
  aura                  Ouvre l’interface graphique d’Aura
  aura switcher         Ouvre le sélecteur rapide de fonds (HUD)
  aura next             Passe au fond suivant
  aura prev             Revient au fond précédent
  aura stop             Arrête la lecture des fonds
  aura toggle-pause     Met en pause ou reprend (0 % GPU en pause)
  aura apply <fichier>  Applique immédiatement une vidéo ou une image
  aura status           Affiche l’état du fond et des écrans
  aura mode <mode>      Change le thème : dark, light, auto, manual
  aura check-update     Vérifie les nouvelles versions
  aura update           Télécharge et installe la dernière mise à jour
  aura --version, -v    Affiche la version d’Aura
  aura help             Affiche cette aide

Astuce : assignez 'aura next' à un raccourci clavier (par ex. Super + W)."#,
        }
    }

    pub fn cli_version(self) -> &'static str {
        match self {
            Language::Es => "Aura Live Wallpaper v",
            Language::En => "Aura Live Wallpaper v",
            Language::Fr => "Aura Live Wallpaper v",
        }
    }

    pub fn cli_error_file_missing(self, path: &str) -> String {
        match self {
            Language::Es => format!("Error: el archivo '{}' no existe.", path),
            Language::En => format!("Error: file '{}' does not exist.", path),
            Language::Fr => format!("Erreur : le fichier '{}' n’existe pas.", path),
        }
    }

    pub fn cli_error_not_regular(self, path: &str) -> String {
        match self {
            Language::Es => format!("Error: '{}' es un directorio o no es un archivo regular.", path),
            Language::En => {
                format!("Error: '{}' is a directory or not a regular file.", path)
            }
            Language::Fr => {
                format!("Erreur : '{}' est un dossier ou n’est pas un fichier régulier.", path)
            }
        }
    }

    pub fn cli_error_unsupported_format(self, ext: &str) -> String {
        match self {
            Language::Es => format!(
                "Error: el formato '.{}' no es compatible. Formatos: mp4, mkv, webm, mov, avi, png, jpg, jpeg, webp, gif.",
                ext
            ),
            Language::En => format!(
                "Error: '.{}' is not supported. Formats: mp4, mkv, webm, mov, avi, png, jpg, jpeg, webp, gif.",
                ext
            ),
            Language::Fr => format!(
                "Erreur : le format '.{}' n’est pas pris en charge. Formats : mp4, mkv, webm, mov, avi, png, jpg, jpeg, webp, gif.",
                ext
            ),
        }
    }

    pub fn cli_error_unknown_command(self, command: &str) -> String {
        match self {
            Language::Es => format!(
                "Comando desconocido: '{}'. Ejecuta 'aura help' para ver las opciones.",
                command
            ),
            Language::En => format!(
                "Unknown command: '{}'. Run 'aura help' to see the available options.",
                command
            ),
            Language::Fr => format!(
                "Commande inconnue : '{}'. Lancez 'aura help' pour voir les options.",
                command
            ),
        }
    }

    pub fn cli_usage_apply(self) -> &'static str {
        match self {
            Language::Es => "Uso: aura apply <ruta_al_video_o_imagen>",
            Language::En => "Usage: aura apply <path_to_video_or_image>",
            Language::Fr => "Utilisation : aura apply <chemin_video_ou_image>",
        }
    }

    pub fn cli_no_wallpapers(self) -> &'static str {
        match self {
            Language::Es => "No se encontraron fondos de pantalla configurados.",
            Language::En => "No configured wallpapers were found.",
            Language::Fr => "Aucun fond d’écran configuré n’a été trouvé.",
        }
    }

    pub fn cli_no_rotation_match(self) -> &'static str {
        match self {
            Language::Es => "Ningún fondo coincide con los filtros de rotación actuales.",
            Language::En => "No wallpaper matches the current rotation filters.",
            Language::Fr => "Aucun fond ne correspond aux filtres de rotation actuels.",
        }
    }

    pub fn cli_wallpaper_changed(self, name: &str, pid: u32) -> String {
        match self {
            Language::Es => format!("✨ Aura: fondo cambiado a '{}' (PID: {})", name, pid),
            Language::En => format!("✨ Aura: wallpaper changed to '{}' (PID: {})", name, pid),
            Language::Fr => format!("✨ Aura : fond changé pour '{}' (PID : {})", name, pid),
        }
    }

    pub fn cli_previous_wallpaper(self, name: &str, pid: u32) -> String {
        match self {
            Language::Es => format!("✨ Aura: fondo anterior activado: '{}' (PID: {})", name, pid),
            Language::En => format!("✨ Aura: previous wallpaper restored: '{}' (PID: {})", name, pid),
            Language::Fr => format!("✨ Aura : fond précédent réactivé : '{}' (PID : {})", name, pid),
        }
    }

    pub fn cli_engine_error(self) -> &'static str {
        match self {
            Language::Es => "Error al cambiar el fondo con mpvpaper.",
            Language::En => "Could not change the wallpaper with mpvpaper.",
            Language::Fr => "Impossible de changer le fond avec mpvpaper.",
        }
    }

    pub fn cli_stopped(self) -> &'static str {
        match self {
            Language::Es => "⏹ Aura: fondo animado detenido.",
            Language::En => "⏹ Aura: animated wallpaper stopped.",
            Language::Fr => "⏹ Aura : fond animé arrêté.",
        }
    }

    pub fn cli_no_mpvpaper(self) -> &'static str {
        match self {
            Language::Es => "No hay ningún fondo de mpvpaper ejecutándose.",
            Language::En => "No mpvpaper wallpaper is currently running.",
            Language::Fr => "Aucun fond mpvpaper n’est en cours d’exécution.",
        }
    }

    pub fn cli_paused(self) -> &'static str {
        match self {
            Language::Es => "⏸ Aura: fondo pausado (0% GPU/CPU).",
            Language::En => "⏸ Aura: wallpaper paused (0% GPU/CPU).",
            Language::Fr => "⏸ Aura : fond en pause (0 % GPU/CPU).",
        }
    }

    pub fn cli_resumed(self) -> &'static str {
        match self {
            Language::Es => "▶ Aura: fondo reanudado.",
            Language::En => "▶ Aura: wallpaper resumed.",
            Language::Fr => "▶ Aura : fond repris.",
        }
    }

    pub fn cli_applied(self, pid: u32, path: &str) -> String {
        match self {
            Language::Es => format!("✨ Aura: fondo aplicado correctamente (PID: {}) -> {}", pid, path),
            Language::En => format!("✨ Aura: wallpaper applied successfully (PID: {}) -> {}", pid, path),
            Language::Fr => format!("✨ Aura : fond appliqué avec succès (PID : {}) -> {}", pid, path),
        }
    }

    pub fn cli_apply_failed(self, error: &str) -> String {
        match self {
            Language::Es => format!("Error al aplicar el fondo: {}", error),
            Language::En => format!("Could not apply the wallpaper: {}", error),
            Language::Fr => format!("Impossible d’appliquer le fond : {}", error),
        }
    }

    pub fn cli_update_checking(self) -> &'static str {
        match self {
            Language::Es => "🔍 Buscando actualizaciones de Aura en GitHub...",
            Language::En => "🔍 Checking for Aura updates on GitHub...",
            Language::Fr => "🔍 Recherche de mises à jour d’Aura sur GitHub...",
        }
    }

    pub fn cli_runtime_error(self, error: &str) -> String {
        match self {
            Language::Es => format!("Error al inicializar el runtime: {}", error),
            Language::En => format!("Could not initialize the runtime: {}", error),
            Language::Fr => format!("Impossible d’initialiser le runtime : {}", error),
        }
    }

    pub fn cli_update_query_failed(self, error: &str) -> String {
        match self {
            Language::Es => format!("No se pudo consultar actualizaciones: {}", error),
            Language::En => format!("Could not check for updates: {}", error),
            Language::Fr => format!("Impossible de vérifier les mises à jour : {}", error),
        }
    }

    pub fn cli_update_available(self, current: &str, latest: &str) -> String {
        match self {
            Language::Es => format!("🚀 ¡Nueva versión disponible! {} -> {}", current, latest),
            Language::En => format!("🚀 New version available! {} -> {}", current, latest),
            Language::Fr => format!("🚀 Nouvelle version disponible ! {} -> {}", current, latest),
        }
    }

    pub fn cli_update_notes(self, notes: &str) -> String {
        match self {
            Language::Es => format!("\nNotas de la versión:\n{}", notes),
            Language::En => format!("\nRelease notes:\n{}", notes),
            Language::Fr => format!("\nNotes de version :\n{}", notes),
        }
    }

    pub fn cli_update_instructions(self) -> &'static str {
        match self {
            Language::Es => "Para actualizar automáticamente, ejecuta:\n  aura update",
            Language::En => "To update automatically, run:\n  aura update",
            Language::Fr => "Pour mettre à jour automatiquement, exécutez :\n  aura update",
        }
    }

    pub fn cli_update_up_to_date(self, version: &str) -> String {
        match self {
            Language::Es => format!("✅ Ya tienes la última versión instalada (v{}).", version),
            Language::En => format!("✅ You already have the latest version installed (v{}).", version),
            Language::Fr => format!("✅ Vous avez déjà la dernière version installée (v{}).", version),
        }
    }

    pub fn cli_update_check_failed(self, error: &str) -> String {
        match self {
            Language::Es => format!("Error al buscar actualizaciones: {}", error),
            Language::En => format!("Error while checking for updates: {}", error),
            Language::Fr => format!("Erreur lors de la recherche de mises à jour : {}", error),
        }
    }

    pub fn cli_update_starting(self) -> &'static str {
        match self {
            Language::Es => "🚀 Comprobando y actualizando Aura...",
            Language::En => "🚀 Checking and updating Aura...",
            Language::Fr => "🚀 Vérification et mise à jour d’Aura...",
        }
    }

    pub fn cli_update_no_package(self, tag: &str) -> String {
        match self {
            Language::Es => format!(
                "No se encontró un paquete verificado compatible con la versión {}.",
                tag
            ),
            Language::En => format!(
                "No verified package compatible with version {} was found.",
                tag
            ),
            Language::Fr => format!(
                "Aucun paquet vérifié compatible avec la version {} n’a été trouvé.",
                tag
            ),
        }
    }

    pub fn cli_update_manual(self) -> &'static str {
        match self {
            Language::Es => "Visita https://github.com/antwny/aura/releases para descargar manualmente.",
            Language::En => "Visit https://github.com/antwny/aura/releases to download it manually.",
            Language::Fr => "Visitez https://github.com/antwny/aura/releases pour télécharger manuellement.",
        }
    }

    pub fn cli_update_downloading(self, tag: &str, current: &str) -> String {
        match self {
            Language::Es => format!("Descargando versión {} (actual: v{})...", tag, current),
            Language::En => format!("Downloading version {} (current: v{})...", tag, current),
            Language::Fr => format!("Téléchargement de la version {} (actuelle : v{})...", tag, current),
        }
    }

    pub fn cli_update_success(self, tag: &str) -> String {
        match self {
            Language::Es => format!("🎉 ¡Aura actualizada correctamente a {}!", tag),
            Language::En => format!("🎉 Aura updated successfully to {}!", tag),
            Language::Fr => format!("🎉 Aura mise à jour avec succès vers {} !", tag),
        }
    }

    pub fn cli_update_location(self, location: &str) -> String {
        match self {
            Language::Es => format!("Ubicación: {}", location),
            Language::En => format!("Location: {}", location),
            Language::Fr => format!("Emplacement : {}", location),
        }
    }

    pub fn cli_update_failed(self, error: &str) -> String {
        match self {
            Language::Es => format!("Error al actualizar: {}", error),
            Language::En => format!("Error while updating: {}", error),
            Language::Fr => format!("Erreur lors de la mise à jour : {}", error),
        }
    }

    pub fn cli_theme_current(self, mode: &str) -> String {
        match self {
            Language::Es => format!("Modo actual de apariencia: {}", mode),
            Language::En => format!("Current appearance mode: {}", mode),
            Language::Fr => format!("Mode d’apparence actuel : {}", mode),
        }
    }

    pub fn cli_theme_usage(self) -> &'static str {
        match self {
            Language::Es => "Uso: aura mode <dark|light|auto|manual>",
            Language::En => "Usage: aura mode <dark|light|auto|manual>",
            Language::Fr => "Utilisation : aura mode <dark|light|auto|manual>",
        }
    }

    pub fn cli_theme_unknown(self, mode: &str) -> String {
        match self {
            Language::Es => format!(
                "Error: modo desconocido '{}'. Opciones: dark, light, auto, manual",
                mode
            ),
            Language::En => format!(
                "Error: unknown mode '{}'. Options: dark, light, auto, manual",
                mode
            ),
            Language::Fr => format!(
                "Erreur : mode inconnu '{}'. Options : dark, light, auto, manual",
                mode
            ),
        }
    }

    pub fn cli_theme_changed(self, mode: &str) -> String {
        match self {
            Language::Es => format!("✨ Aura: modo de apariencia cambiado a {}", mode),
            Language::En => format!("✨ Aura: appearance mode changed to {}", mode),
            Language::Fr => format!("✨ Aura : mode d’apparence changé pour {}", mode),
        }
    }

    pub fn cli_theme_mode_name(self, mode: crate::config::ThemeMode) -> &'static str {
        let labels = self.cli_labels();
        match mode {
            crate::config::ThemeMode::Auto => labels.theme_auto,
            crate::config::ThemeMode::Dark => labels.theme_dark,
            crate::config::ThemeMode::Light => labels.theme_light,
            crate::config::ThemeMode::Manual => labels.theme_manual,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_labels_are_localized() {
        assert!(Language::Es.cli_labels().status_title.contains("Estado"));
        assert!(Language::En.cli_labels().status_title.contains("System Status"));
        assert!(Language::Fr.cli_labels().status_title.contains("État"));
        for language in [Language::Es, Language::En, Language::Fr] {
            assert!(!language.cli_help().is_empty());
            assert!(!language.cli_labels().auto_pause_partial.is_empty());
            assert!(language
                .cli_error_file_missing("/tmp/x")
                .contains("/tmp/x"));
        }
    }
}
