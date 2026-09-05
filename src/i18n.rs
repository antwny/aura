use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "es")]
    Es,
    #[serde(rename = "en")]
    En,
}

impl Default for Language {
    fn default() -> Self {
        detect_system_language()
    }
}

pub fn detect_system_language() -> Language {
    if let Ok(lang) = std::env::var("LANG").or_else(|_| std::env::var("LC_MESSAGES")) {
        if lang.to_lowercase().starts_with("es") {
            return Language::Es;
        }
    }
    Language::En
}

impl Language {
    pub fn from_code(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "es" | "spanish" | "español" => Language::Es,
            _ => Language::En,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Language::Es => "es",
            Language::En => "en",
        }
    }

    // --- Navigation ---
    pub fn nav_library(&self) -> &'static str {
        match self {
            Language::Es => "Biblioteca",
            Language::En => "Library",
        }
    }

    pub fn nav_explore(&self) -> &'static str {
        match self {
            Language::Es => "Explorar",
            Language::En => "Explore",
        }
    }

    pub fn nav_monitors(&self) -> &'static str {
        match self {
            Language::Es => "Pantallas",
            Language::En => "Displays",
        }
    }

    pub fn nav_settings(&self) -> &'static str {
        match self {
            Language::Es => "Ajustes",
            Language::En => "Settings",
        }
    }

    pub fn nav_about(&self) -> &'static str {
        match self {
            Language::Es => "Acerca de",
            Language::En => "About",
        }
    }

    // --- Header ---
    pub fn header_wallpapers_count(&self, count: usize) -> String {
        match self {
            Language::Es => format!("({} fondos)", count),
            Language::En => format!("({} wallpapers)", count),
        }
    }

    pub fn header_add_video(&self) -> &'static str {
        match self {
            Language::Es => "+ Añadir Video",
            Language::En => "+ Add Video",
        }
    }

    pub fn header_add_folder(&self) -> &'static str {
        match self {
            Language::Es => "+ Carpeta",
            Language::En => "+ Folder",
        }
    }

    // --- Library View ---
    pub fn library_search_placeholder(&self) -> &'static str {
        match self {
            Language::Es => "Buscar fondos por nombre...",
            Language::En => "Search wallpapers by name...",
        }
    }

    pub fn library_empty_title(&self) -> &'static str {
        match self {
            Language::Es => "Tu Biblioteca de Fondos está vacía",
            Language::En => "Your Wallpaper Library is empty",
        }
    }

    pub fn library_empty_desc(&self) -> &'static str {
        match self {
            Language::Es => "Añade videos o carpetas para empezar a personalizar tu escritorio:",
            Language::En => "Add videos or folders to start customizing your desktop:",
        }
    }

    pub fn library_btn_add_video(&self) -> &'static str {
        self.header_add_video()
    }

    pub fn library_btn_add_folder(&self) -> &'static str {
        self.header_add_folder()
    }

    pub fn library_extracting_frame(&self) -> &'static str {
        match self {
            Language::Es => "Extrayendo fotograma...",
            Language::En => "Extracting frame...",
        }
    }

    pub fn library_active(&self) -> &'static str {
        match self {
            Language::Es => "★ Activo",
            Language::En => "★ Active",
        }
    }

    pub fn library_apply(&self) -> &'static str {
        match self {
            Language::Es => "Aplicar",
            Language::En => "Apply",
        }
    }

    // --- Monitors View ---
    pub fn monitors_title(&self) -> &'static str {
        match self {
            Language::Es => "Visualizador Interactivo de Pantallas",
            Language::En => "Interactive Display Visualizer",
        }
    }

    pub fn monitors_desc(&self) -> &'static str {
        match self {
            Language::Es => "Representación a escala de tus pantallas detectadas en COSMIC. Configura fondos y escalado independiente:",
            Language::En => "True-to-scale layout of detected COSMIC displays. Configure wallpapers and scaling independently:",
        }
    }

    pub fn monitors_screen_name(&self, name: &str) -> String {
        match self {
            Language::Es => format!("Pantalla {}", name),
            Language::En => format!("Display {}", name),
        }
    }

    pub fn monitors_screen_idle(&self, name: &str) -> String {
        match self {
            Language::Es => format!("Pantalla {}\n(Escritorio COSMIC)", name),
            Language::En => format!("Display {}\n(COSMIC Desktop)", name),
        }
    }

    pub fn monitors_stop(&self) -> &'static str {
        match self {
            Language::Es => "Detener",
            Language::En => "Stop",
        }
    }

    // --- Bottom Bar ("Now Playing") ---
    pub fn bar_idle_title(&self) -> &'static str {
        match self {
            Language::Es => "Ningún fondo en reproducción",
            Language::En => "No wallpaper playing",
        }
    }

    pub fn bar_idle_desc(&self) -> &'static str {
        match self {
            Language::Es => "Escritorio COSMIC estándar",
            Language::En => "Standard COSMIC desktop",
        }
    }

    pub fn bar_active_title_default(&self) -> &'static str {
        match self {
            Language::Es => "Fondo Activo",
            Language::En => "Active Wallpaper",
        }
    }

    pub fn bar_display_label(&self, outputs: &str) -> String {
        match self {
            Language::Es => format!("Pantalla: {}", outputs),
            Language::En => format!("Display: {}", outputs),
        }
    }

    pub fn bar_resume(&self) -> &'static str {
        match self {
            Language::Es => "Reanudar",
            Language::En => "Resume",
        }
    }

    pub fn bar_pause(&self) -> &'static str {
        match self {
            Language::Es => "Pausar",
            Language::En => "Pause",
        }
    }

    pub fn bar_muted(&self) -> &'static str {
        match self {
            Language::Es => "Silenciado",
            Language::En => "Muted",
        }
    }

    pub fn bar_audio_active(&self) -> &'static str {
        match self {
            Language::Es => "Sonido Activo",
            Language::En => "Audio On",
        }
    }

    pub fn bar_stop(&self) -> &'static str {
        match self {
            Language::Es => "Detener",
            Language::En => "Stop",
        }
    }

    pub fn bar_gpu_accel(&self, hwdec: &str) -> String {
        match self {
            Language::Es => format!("Aceleración GPU: {}", hwdec),
            Language::En => format!("GPU Acceleration: {}", hwdec),
        }
    }

    pub fn bar_wayland_tag(&self) -> &'static str {
        "Wayland Layer-Shell"
    }

    // --- Settings View ---
    pub fn settings_title(&self) -> &'static str {
        match self {
            Language::Es => "Ajustes del Sistema",
            Language::En => "System Settings",
        }
    }

    pub fn settings_lang_title(&self) -> &'static str {
        match self {
            Language::Es => "Idioma / Language",
            Language::En => "Language / Idioma",
        }
    }

    pub fn settings_lang_desc(&self) -> &'static str {
        match self {
            Language::Es => "Selecciona el idioma de la aplicación. El cambio se aplica inmediatamente sin reiniciar.",
            Language::En => "Select the application language. Changes take effect instantly without restarting.",
        }
    }

    pub fn settings_integration_title(&self) -> &'static str {
        match self {
            Language::Es => "Integración y Rendimiento",
            Language::En => "Integration & Performance",
        }
    }

    pub fn settings_autostart(&self) -> &'static str {
        match self {
            Language::Es => "Iniciar fondo animado automáticamente al iniciar sesión en Pop!_OS",
            Language::En => "Launch animated wallpaper automatically upon login to Pop!_OS",
        }
    }

    pub fn settings_smart_pause(&self) -> &'static str {
        match self {
            Language::Es => "Pausar automáticamente en juegos y ventanas a pantalla completa (Smart Pause)",
            Language::En => "Automatically pause in fullscreen games and windows (Smart Pause)",
        }
    }

    pub fn settings_keep_running(&self) -> &'static str {
        match self {
            Language::Es => "Minimizar a la barra superior (bandeja del sistema) al cerrar la ventana",
            Language::En => "Minimize to the top bar (system tray) when closing window",
        }
    }

    pub fn settings_auto_theme(&self) -> &'static str {
        match self {
            Language::Es => "Sincronizar color de acento del sistema con el fondo (Auto-Tema COSMIC)",
            Language::En => "Synchronize system accent color with active wallpaper (COSMIC Auto-Theme)",
        }
    }

    pub fn settings_auto_dark(&self) -> &'static str {
        match self {
            Language::Es => "Cambiar automáticamente entre Modo Oscuro y Claro según la claridad del video",
            Language::En => "Automatically toggle Dark and Light mode based on video luminance",
        }
    }

    pub fn settings_monitored_folders(&self) -> &'static str {
        match self {
            Language::Es => "Carpetas Monitoreadas",
            Language::En => "Monitored Folders",
        }
    }

    pub fn settings_btn_add_folder(&self) -> &'static str {
        match self {
            Language::Es => "+ Añadir Carpeta",
            Language::En => "+ Add Folder",
        }
    }

    pub fn settings_btn_delete(&self) -> &'static str {
        match self {
            Language::Es => "Eliminar",
            Language::En => "Remove",
        }
    }

    pub fn settings_persistence_title(&self) -> &'static str {
        match self {
            Language::Es => "Persistencia en segundo plano",
            Language::En => "Background Persistence",
        }
    }

    pub fn settings_persistence_desc(&self) -> &'static str {
        match self {
            Language::Es => "Aura funciona como centro de control. Al cerrar esta ventana, tu fondo animado continuará reproduciéndose sin problemas en tu compositor Wayland a través de mpvpaper, liberando el 100% de la memoria de la interfaz.",
            Language::En => "Aura serves as a control center. When this window is closed, your live wallpaper keeps playing seamlessly in your Wayland compositor via mpvpaper, freeing 100% of the UI memory.",
        }
    }

    // --- About View ---
    pub fn about_tagline(&self) -> &'static str {
        match self {
            Language::Es => "Gestor Nativo de Fondos Animados para COSMIC Desktop",
            Language::En => "Native Live Wallpaper Manager for COSMIC Desktop",
        }
    }

    pub fn about_version_info(&self) -> &'static str {
        match self {
            Language::Es => concat!("Versión ", env!("CARGO_PKG_VERSION"), " • Pop!_OS 24.04 LTS"),
            Language::En => concat!("Version ", env!("CARGO_PKG_VERSION"), " • Pop!_OS 24.04 LTS"),
        }
    }

    pub fn about_details_title(&self) -> &'static str {
        match self {
            Language::Es => "Detalles de la Aplicación",
            Language::En => "Application Details",
        }
    }

    pub fn about_developer_lbl(&self) -> &'static str {
        match self {
            Language::Es => "Desarrollador:",
            Language::En => "Developer:",
        }
    }

    pub fn about_architecture_lbl(&self) -> &'static str {
        match self {
            Language::Es => "Arquitectura:",
            Language::En => "Architecture:",
        }
    }

    pub fn about_license_lbl(&self) -> &'static str {
        match self {
            Language::Es => "Licencia:",
            Language::En => "License:",
        }
    }

    pub fn about_summary_desc(&self) -> &'static str {
        match self {
            Language::Es => "Aura fue concebido para transformar la experiencia de fondos de pantalla animados en Linux. Aprovechando el poder de Rust y libcosmic, elimina los congelamientos tradicionales y consume menos de 25 MB de memoria RAM con decodificación completa por GPU y sincronización automática de color.",
            Language::En => "Aura was engineered to redefine animated live wallpapers on Linux. Harnessing native Rust and libcosmic, it eliminates freezes and consumes under 25 MB of RAM with full GPU hardware acceleration and dynamic cosmic color synchronization.",
        }
    }

    pub fn about_github_btn(&self) -> &'static str {
        match self {
            Language::Es => "Repositorio en GitHub",
            Language::En => "GitHub Repository",
        }
    }

    pub fn about_youtube_btn(&self) -> &'static str {
        match self {
            Language::Es => "Canal de YouTube",
            Language::En => "YouTube Channel",
        }
    }

    pub fn about_donation_lbl(&self) -> &'static str {
        match self {
            Language::Es => "Donaciones:",
            Language::En => "Donations:",
        }
    }

    pub fn about_donate_btn(&self) -> &'static str {
        match self {
            Language::Es => "💖 Donar (PayPal)",
            Language::En => "💖 Donate (PayPal)",
        }
    }

    // --- Explore View (Online Catalog) ---
    pub fn explore_source_bing(&self) -> &'static str {
        match self {
            Language::Es => "🌅 Bing del Día",
            Language::En => "🌅 Bing Daily",
        }
    }

    pub fn explore_source_wallhaven(&self) -> &'static str {
        match self {
            Language::Es => "🌌 Wallhaven (Top 4K)",
            Language::En => "🌌 Wallhaven (Top 4K)",
        }
    }

    pub fn explore_btn_download(&self) -> &'static str {
        match self {
            Language::Es => "📥 Descargar",
            Language::En => "📥 Download",
        }
    }

    pub fn explore_btn_downloading(&self) -> &'static str {
        match self {
            Language::Es => "⏳ Descargando...",
            Language::En => "⏳ Downloading...",
        }
    }

    pub fn explore_btn_apply(&self) -> &'static str {
        match self {
            Language::Es => "✨ Aplicar",
            Language::En => "✨ Apply",
        }
    }

    pub fn explore_badge_active(&self) -> &'static str {
        match self {
            Language::Es => "✓ En Uso",
            Language::En => "✓ Active",
        }
    }

    pub fn explore_featured_today(&self) -> &'static str {
        match self {
            Language::Es => "⭐ Foto Destacada de Hoy (4K UHD)",
            Language::En => "⭐ Daily Featured Photo (4K UHD)",
        }
    }

    pub fn explore_recent_title(&self) -> &'static str {
        match self {
            Language::Es => "Galería & Archivo Online",
            Language::En => "Online Gallery & Archive",
        }
    }

    pub fn explore_loading(&self) -> &'static str {
        match self {
            Language::Es => "Cargando catálogo en línea...",
            Language::En => "Loading online catalog...",
        }
    }

    pub fn explore_error_prefix(&self) -> &'static str {
        match self {
            Language::Es => "No se pudo conectar con el catálogo en línea:",
            Language::En => "Could not connect to online catalog:",
        }
    }

    pub fn explore_btn_retry(&self) -> &'static str {
        match self {
            Language::Es => "🔄 Reintentar conexión",
            Language::En => "🔄 Retry connection",
        }
    }

    pub fn explore_toast_downloaded(&self) -> &'static str {
        match self {
            Language::Es => "¡Fondo descargado y guardado en tu biblioteca!",
            Language::En => "Wallpaper downloaded and saved to your library!",
        }
    }

    // --- Tray (StatusNotifierItem) ---
    pub fn tray_open(&self) -> &'static str {
        match self {
            Language::Es => "Abrir Aura",
            Language::En => "Open Aura",
        }
    }

    pub fn tray_pause(&self) -> &'static str {
        match self {
            Language::Es => "Pausar Fondo",
            Language::En => "Pause Wallpaper",
        }
    }

    pub fn tray_resume(&self) -> &'static str {
        match self {
            Language::Es => "Reanudar Fondo",
            Language::En => "Resume Wallpaper",
        }
    }

    pub fn tray_next(&self) -> &'static str {
        match self {
            Language::Es => "Siguiente Fondo",
            Language::En => "Next Wallpaper",
        }
    }

    pub fn tray_stop(&self) -> &'static str {
        match self {
            Language::Es => "Detener Fondo",
            Language::En => "Stop Wallpaper",
        }
    }

    pub fn tray_quit(&self) -> &'static str {
        match self {
            Language::Es => "Salir de Aura",
            Language::En => "Quit Aura",
        }
    }

    pub fn tray_tooltip_idle(&self) -> &'static str {
        match self {
            Language::Es => "Sin fondo en reproducción",
            Language::En => "No wallpaper playing",
        }
    }

    pub fn tray_tooltip_playing(&self, name: &str) -> String {
        match self {
            Language::Es => format!("Reproduciendo: {}", name),
            Language::En => format!("Playing: {}", name),
        }
    }

    // --- Status Messages / Toasts ---
    pub fn status_lang_changed(&self) -> &'static str {
        match self {
            Language::Es => "Idioma cambiado a Español",
            Language::En => "Language switched to English",
        }
    }

    pub fn status_applied(&self, output: &str, name: &str) -> String {
        match self {
            Language::Es => format!("Fondo aplicado en {}: '{}'", output, name),
            Language::En => format!("Wallpaper applied on {}: '{}'", output, name),
        }
    }

    pub fn status_paused(&self) -> &'static str {
        match self {
            Language::Es => "Fondo pausado (0% GPU/CPU)",
            Language::En => "Wallpaper paused (0% GPU/CPU)",
        }
    }

    pub fn status_resumed(&self) -> &'static str {
        match self {
            Language::Es => "Fondo reanudado",
            Language::En => "Wallpaper resumed",
        }
    }

    pub fn status_stopped_output(&self, output: &str) -> String {
        match self {
            Language::Es => format!("Fondo detenido en {}", output),
            Language::En => format!("Wallpaper stopped on {}", output),
        }
    }

    pub fn status_stopped_all(&self) -> &'static str {
        match self {
            Language::Es => "Todos los fondos han sido detenidos",
            Language::En => "All wallpapers have been stopped",
        }
    }

    pub fn status_muted(&self) -> &'static str {
        match self {
            Language::Es => "Audio silenciado",
            Language::En => "Audio muted",
        }
    }

    pub fn status_unmuted(&self) -> &'static str {
        match self {
            Language::Es => "Audio activado",
            Language::En => "Audio enabled",
        }
    }

    pub fn status_folder_removed(&self, folder: &str) -> String {
        match self {
            Language::Es => format!("Carpeta eliminada: {}", folder),
            Language::En => format!("Folder removed: {}", folder),
        }
    }

    pub fn status_library_refreshed(&self) -> &'static str {
        match self {
            Language::Es => "Biblioteca actualizada",
            Language::En => "Library updated",
        }
    }

    pub fn status_video_added(&self, name: &str) -> String {
        match self {
            Language::Es => format!("Video añadido: {}", name),
            Language::En => format!("Video added: {}", name),
        }
    }

    pub fn status_folder_added(&self, folder: &str) -> String {
        match self {
            Language::Es => format!("Carpeta añadida: {}", folder),
            Language::En => format!("Folder added: {}", folder),
        }
    }

    pub fn status_videos_dropped(&self) -> &'static str {
        match self {
            Language::Es => "Videos añadidos a la biblioteca",
            Language::En => "Videos added to library",
        }
    }

    pub fn status_topology_updated(&self, count: usize) -> String {
        match self {
            Language::Es => format!("Topología de pantallas actualizada ({} monitores)", count),
            Language::En => format!("Display topology updated ({} displays)", count),
        }
    }

    pub fn status_game_paused(&self) -> &'static str {
        match self {
            Language::Es => "Modo Juego: Fondo pausado para máximo rendimiento",
            Language::En => "Game Mode: Wallpaper paused for maximum performance",
        }
    }

    pub fn status_game_resumed(&self) -> &'static str {
        match self {
            Language::Es => "Juego minimizado: Fondo reanudado",
            Language::En => "Game minimized: Wallpaper resumed",
        }
    }

    pub fn dialog_pick_video(&self) -> &'static str {
        match self {
            Language::Es => "Seleccionar Video de Fondo",
            Language::En => "Select Wallpaper Video",
        }
    }

    pub fn dialog_pick_folder(&self) -> &'static str {
        match self {
            Language::Es => "Seleccionar Carpeta con Videos",
            Language::En => "Select Folder with Videos",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_code() {
        assert_eq!(Language::from_code("es"), Language::Es);
        assert_eq!(Language::from_code("ES"), Language::Es);
        assert_eq!(Language::from_code("spanish"), Language::Es);
        assert_eq!(Language::from_code("español"), Language::Es);
        assert_eq!(Language::from_code("en"), Language::En);
        assert_eq!(Language::from_code("EN"), Language::En);
        assert_eq!(Language::from_code("english"), Language::En);
        assert_eq!(Language::from_code("fr"), Language::En);
        assert_eq!(Language::from_code(""), Language::En);
    }

    #[test]
    fn test_language_code() {
        assert_eq!(Language::Es.code(), "es");
        assert_eq!(Language::En.code(), "en");
    }

    #[test]
    fn test_translations_non_empty() {
        for lang in &[Language::Es, Language::En] {
            assert!(!lang.nav_library().is_empty());
            assert!(!lang.nav_explore().is_empty());
            assert!(!lang.nav_monitors().is_empty());
            assert!(!lang.nav_settings().is_empty());
            assert!(!lang.nav_about().is_empty());
            assert!(!lang.header_add_video().is_empty());
            assert!(!lang.header_add_folder().is_empty());
            assert!(!lang.library_search_placeholder().is_empty());
            assert!(!lang.library_empty_title().is_empty());
            assert!(!lang.settings_title().is_empty());
            assert!(!lang.about_tagline().is_empty());
            assert!(!lang.tray_open().is_empty());
            assert!(!lang.tray_quit().is_empty());
            assert!(!lang.status_paused().is_empty());
            assert!(!lang.status_resumed().is_empty());
        }
    }

    #[test]
    fn test_dynamic_formatting() {
        assert_eq!(Language::Es.header_wallpapers_count(5), "(5 fondos)");
        assert_eq!(Language::En.header_wallpapers_count(5), "(5 wallpapers)");
        assert!(Language::Es.status_applied("DP-1", "ocean.mp4").contains("DP-1"));
        assert!(Language::En.status_applied("DP-1", "ocean.mp4").contains("ocean.mp4"));
    }
}
