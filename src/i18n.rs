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
            Language::Es => "Añadir Video",
            Language::En => "Add Video",
        }
    }

    pub fn header_add_folder(&self) -> &'static str {
        match self {
            Language::Es => "Añadir Carpeta",
            Language::En => "Add Folder",
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
            Language::Es => "En uso",
            Language::En => "Active",
        }
    }

    pub fn library_apply(&self) -> &'static str {
        match self {
            Language::Es => "Aplicar",
            Language::En => "Apply",
        }
    }

    pub fn library_filter_all(&self) -> &'static str {
        match self {
            Language::Es => "Todos",
            Language::En => "All",
        }
    }

    pub fn library_filter_live(&self) -> &'static str {
        match self {
            Language::Es => "Animados",
            Language::En => "Live",
        }
    }

    pub fn library_filter_static(&self) -> &'static str {
        match self {
            Language::Es => "Estáticos",
            Language::En => "Static",
        }
    }

    pub fn library_filter_downloaded(&self) -> &'static str {
        match self {
            Language::Es => "Descargados",
            Language::En => "Downloaded",
        }
    }

    pub fn library_filter_empty(&self) -> &'static str {
        match self {
            Language::Es => "No se encontraron fondos en esta categoría o búsqueda",
            Language::En => "No wallpapers found in this category or search",
        }
    }

    #[allow(dead_code)]
    pub fn library_btn_delete(&self) -> &'static str {
        match self {
            Language::Es => "Eliminar",
            Language::En => "Delete",
        }
    }

    pub fn library_deleted_toast(&self) -> &'static str {
        match self {
            Language::Es => "Fondo eliminado de la biblioteca",
            Language::En => "Wallpaper removed from library",
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
            Language::Es => "Añadir Carpeta",
            Language::En => "Add Folder",
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

    #[allow(dead_code)]
    pub fn about_summary_desc(&self) -> &'static str {
        match self {
            Language::Es => "Aura fue concebido para transformar la experiencia de fondos de pantalla animados en Linux. Aprovechando el poder nativo de Rust y libcosmic, ofrece un rendimiento fluido de 60 FPS con huella de memoria ultraligera, decodificación completa por GPU y sincronización automática de color.",
            Language::En => "Aura was engineered to redefine animated live wallpapers on Linux. Harnessing native Rust and libcosmic, it delivers smooth 60 FPS playback with minimal memory footprint, full GPU hardware acceleration, and dynamic cosmic color synchronization.",
        }
    }

    pub fn about_github_btn(&self) -> &'static str {
        match self {
            Language::Es => "Github",
            Language::En => "Github",
        }
    }

    pub fn about_youtube_btn(&self) -> &'static str {
        match self {
            Language::Es => "YouTube",
            Language::En => "YouTube",
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
            Language::Es => "Donar (PayPal)",
            Language::En => "Donate (PayPal)",
        }
    }

    pub fn about_updates_title(&self) -> &'static str {
        match self {
            Language::Es => "Actualizaciones de Software",
            Language::En => "Software Updates",
        }
    }

    pub fn about_check_updates_btn(&self) -> &'static str {
        match self {
            Language::Es => "Buscar actualizaciones",
            Language::En => "Check for updates",
        }
    }

    pub fn about_checking_updates(&self) -> &'static str {
        match self {
            Language::Es => "Buscando actualizaciones en GitHub...",
            Language::En => "Checking for updates on GitHub...",
        }
    }

    pub fn about_up_to_date(&self) -> &'static str {
        match self {
            Language::Es => "Aura está al día con la última versión.",
            Language::En => "Aura is up to date.",
        }
    }

    pub fn about_update_available(&self) -> &'static str {
        match self {
            Language::Es => "Nueva versión disponible:",
            Language::En => "New version available:",
        }
    }

    pub fn about_update_now_btn(&self) -> &'static str {
        match self {
            Language::Es => "Actualizar a",
            Language::En => "Update to",
        }
    }

    pub fn about_updating(&self) -> &'static str {
        match self {
            Language::Es => "Descargando e instalando actualización...",
            Language::En => "Downloading and installing update...",
        }
    }

    pub fn about_restart_btn(&self) -> &'static str {
        match self {
            Language::Es => "Reiniciar Aura",
            Language::En => "Restart Aura",
        }
    }

    pub fn about_flatpak_managed(&self) -> &'static str {
        match self {
            Language::Es => "Versión Flatpak: las actualizaciones se gestionan automáticamente a través de COSMIC App Store o Flathub.",
            Language::En => "Flatpak edition: updates are managed via COSMIC App Store or Flathub.",
        }
    }

    pub fn toast_update_available_action(&self) -> &'static str {
        match self {
            Language::Es => "Actualizar",
            Language::En => "Update",
        }
    }

    pub fn toast_restart_action(&self) -> &'static str {
        match self {
            Language::Es => "Reiniciar",
            Language::En => "Restart",
        }
    }

    // --- Explore View (Online Catalog) ---
    pub fn explore_source_bing(&self) -> &'static str {
        match self {
            Language::Es => "Bing del Día (4K)",
            Language::En => "Bing Daily (4K)",
        }
    }

    pub fn explore_source_wallhaven(&self) -> &'static str {
        match self {
            Language::Es => "Wallhaven (Top 4K)",
            Language::En => "Wallhaven (Top 4K)",
        }
    }

    pub fn explore_btn_download(&self) -> &'static str {
        match self {
            Language::Es => "Descargar",
            Language::En => "Download",
        }
    }

    pub fn explore_btn_downloading(&self) -> &'static str {
        match self {
            Language::Es => "Descargando...",
            Language::En => "Downloading...",
        }
    }

    pub fn explore_btn_apply(&self) -> &'static str {
        match self {
            Language::Es => "Aplicar",
            Language::En => "Apply",
        }
    }

    pub fn explore_badge_active(&self) -> &'static str {
        match self {
            Language::Es => "En Uso",
            Language::En => "Active",
        }
    }

    pub fn explore_featured_today(&self) -> &'static str {
        match self {
            Language::Es => "Foto destacada de hoy en 4K UHD",
            Language::En => "Daily featured photo in 4K UHD",
        }
    }

    pub fn explore_recent_title(&self) -> &'static str {
        match self {
            Language::Es => "Galería y archivo 4K",
            Language::En => "4K gallery & archive",
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
            Language::Es => "Reintentar conexión",
            Language::En => "Retry connection",
        }
    }

    pub fn explore_toast_downloaded(&self) -> &'static str {
        match self {
            Language::Es => "Fondo descargado con éxito",
            Language::En => "Wallpaper downloaded successfully",
        }
    }

    pub fn open_in_file_manager(&self) -> &'static str {
        match self {
            Language::Es => "Abrir carpeta",
            Language::En => "Open folder",
        }
    }

    pub fn toast_show_in_files(&self) -> &'static str {
        match self {
            Language::Es => "Ver en archivos",
            Language::En => "Show in files",
        }
    }

    pub fn toast_folder_opened(&self) -> &'static str {
        match self {
            Language::Es => "Carpeta de fondos abierta en el Gestor de Archivos",
            Language::En => "Wallpapers folder opened in File Manager",
        }
    }

    pub fn toast_folder_open_failed(&self) -> &'static str {
        match self {
            Language::Es => "No se pudo abrir la carpeta en el Gestor de Archivos",
            Language::En => "Could not open folder in File Manager",
        }
    }

    pub fn explore_btn_load_more(&self) -> &'static str {
        match self {
            Language::Es => "Cargar más fondos",
            Language::En => "Load more wallpapers",
        }
    }

    pub fn explore_btn_loading_more(&self) -> &'static str {
        match self {
            Language::Es => "Cargando más fondos...",
            Language::En => "Loading more wallpapers...",
        }
    }

    pub fn explore_search_placeholder(&self) -> &'static str {
        match self {
            Language::Es => "Buscar en Wallhaven (ej: anime, cyberpunk, nature)...",
            Language::En => "Search Wallhaven (e.g. anime, cyberpunk, nature)...",
        }
    }

    pub fn explore_cat_all(&self) -> &'static str {
        match self {
            Language::Es => "Todos",
            Language::En => "All",
        }
    }

    pub fn explore_cat_anime(&self) -> &'static str {
        match self {
            Language::Es => "Anime",
            Language::En => "Anime",
        }
    }

    pub fn explore_cat_general(&self) -> &'static str {
        match self {
            Language::Es => "Naturaleza / General",
            Language::En => "Nature / General",
        }
    }

    pub fn explore_sort_top(&self) -> &'static str {
        match self {
            Language::Es => "Más Votados",
            Language::En => "Top Rated",
        }
    }

    pub fn explore_sort_hot(&self) -> &'static str {
        match self {
            Language::Es => "Tendencias",
            Language::En => "Hot",
        }
    }

    pub fn explore_sort_random(&self) -> &'static str {
        match self {
            Language::Es => "Aleatorio",
            Language::En => "Random",
        }
    }

    pub fn explore_res_all(&self) -> &'static str {
        match self {
            Language::Es => "Resolución: Todas",
            Language::En => "Resolution: All",
        }
    }

    pub fn explore_res_4k(&self) -> &'static str {
        "4K UHD"
    }

    pub fn explore_res_2k(&self) -> &'static str {
        "2K QHD"
    }

    pub fn explore_res_ultrawide(&self) -> &'static str {
        "Ultrawide 21:9"
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

    #[allow(dead_code)]
    pub fn library_btn_remove_custom(&self) -> &'static str {
        match self {
            Language::Es => "Quitar",
            Language::En => "Remove",
        }
    }

    #[allow(dead_code)]
    pub fn library_btn_delete_download(&self) -> &'static str {
        match self {
            Language::Es => "Eliminar",
            Language::En => "Delete",
        }
    }

    pub fn library_removed_toast(&self) -> &'static str {
        match self {
            Language::Es => "Fondo quitado de la biblioteca",
            Language::En => "Wallpaper removed from library",
        }
    }

    pub fn library_target_output_label(&self) -> &'static str {
        match self {
            Language::Es => "Pantalla destino:",
            Language::En => "Target Display:",
        }
    }

    pub fn monitors_apply_to_this(&self) -> &'static str {
        match self {
            Language::Es => "Elegir pantalla",
            Language::En => "Select display",
        }
    }

    pub fn monitors_all_displays(&self) -> &'static str {
        match self {
            Language::Es => "Todas las pantallas",
            Language::En => "All Displays",
        }
    }

    pub fn settings_rotation_title(&self) -> &'static str {
        match self {
            Language::Es => "Rotación Automática de Fondos",
            Language::En => "Playlist Auto-Rotation",
        }
    }

    pub fn settings_rotation_desc(&self) -> &'static str {
        match self {
            Language::Es => "Cambia periódicamente de fondo entre los elementos de la biblioteca",
            Language::En => "Periodically switch wallpapers from your library collection",
        }
    }

    pub fn settings_rotation_toggle(&self) -> &'static str {
        match self {
            Language::Es => "Activar rotación automática",
            Language::En => "Enable automatic rotation",
        }
    }

    pub fn status_rotation_enabled(&self) -> &'static str {
        match self {
            Language::Es => "Rotación automática activada",
            Language::En => "Automatic rotation enabled",
        }
    }

    pub fn status_rotation_disabled(&self) -> &'static str {
        match self {
            Language::Es => "Rotación automática desactivada",
            Language::En => "Automatic rotation disabled",
        }
    }

    pub fn settings_interval_label(&self) -> &'static str {
        match self {
            Language::Es => "Intervalo de cambio:",
            Language::En => "Switch Interval:",
        }
    }

    pub fn settings_order_label(&self) -> &'static str {
        match self {
            Language::Es => "Orden de reproducción:",
            Language::En => "Playback Order:",
        }
    }

    pub fn settings_order_random(&self) -> &'static str {
        match self {
            Language::Es => "Aleatorio",
            Language::En => "Random",
        }
    }

    pub fn settings_order_seq(&self) -> &'static str {
        match self {
            Language::Es => "Secuencial",
            Language::En => "Sequential",
        }
    }

    pub fn settings_hwdec_title(&self) -> &'static str {
        match self {
            Language::Es => "Aceleración de Hardware (GPU)",
            Language::En => "Hardware Acceleration (GPU)",
        }
    }

    pub fn settings_hwdec_desc(&self) -> &'static str {
        match self {
            Language::Es => "Configura el decodificador de video para mpv (VA-API / NVDEC)",
            Language::En => "Configure the video hardware decoder for mpv (VA-API / NVDEC)",
        }
    }

    pub fn settings_battery_title(&self) -> &'static str {
        match self {
            Language::Es => "Ahorro de Batería en Portátiles",
            Language::En => "Laptop Battery Saver",
        }
    }

    pub fn settings_battery_desc(&self) -> &'static str {
        match self {
            Language::Es => "Suspende automáticamente la reproducción al usar la batería del portátil",
            Language::En => "Automatically suspend playback when running on laptop battery",
        }
    }

    pub fn status_battery_paused(&self) -> &'static str {
        match self {
            Language::Es => "Ahorro de batería: Fondo pausado (0% GPU)",
            Language::En => "Battery saver: Wallpaper paused (0% GPU)",
        }
    }

    pub fn status_battery_resumed(&self) -> &'static str {
        match self {
            Language::Es => "Alimentación por CA conectada: Fondo reanudado",
            Language::En => "AC power connected: Wallpaper resumed",
        }
    }

    pub fn bar_volume(&self) -> &'static str {
        match self {
            Language::Es => "Volumen",
            Language::En => "Volume",
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
            assert!(!lang.explore_btn_load_more().is_empty());
            assert!(!lang.explore_btn_loading_more().is_empty());
            assert!(!lang.explore_search_placeholder().is_empty());
            assert!(!lang.explore_cat_all().is_empty());
            assert!(!lang.explore_cat_anime().is_empty());
            assert!(!lang.explore_cat_general().is_empty());
            assert!(!lang.explore_sort_top().is_empty());
            assert!(!lang.explore_sort_hot().is_empty());
            assert!(!lang.explore_sort_random().is_empty());
            assert!(!lang.library_filter_all().is_empty());
            assert!(!lang.library_filter_live().is_empty());
            assert!(!lang.library_filter_static().is_empty());
            assert!(!lang.library_filter_downloaded().is_empty());
            assert!(!lang.library_filter_empty().is_empty());
            assert!(!lang.library_btn_delete().is_empty());
            assert!(!lang.library_deleted_toast().is_empty());
            assert!(!lang.explore_res_all().is_empty());
            assert!(!lang.explore_res_4k().is_empty());
            assert!(!lang.explore_res_2k().is_empty());
            assert!(!lang.explore_res_ultrawide().is_empty());
            assert!(!lang.library_btn_remove_custom().is_empty());
            assert!(!lang.library_btn_delete_download().is_empty());
            assert!(!lang.library_removed_toast().is_empty());
            assert!(!lang.library_target_output_label().is_empty());
            assert!(!lang.monitors_apply_to_this().is_empty());
            assert!(!lang.monitors_all_displays().is_empty());
            assert!(!lang.settings_rotation_title().is_empty());
            assert!(!lang.settings_rotation_desc().is_empty());
            assert!(!lang.settings_rotation_toggle().is_empty());
            assert!(!lang.settings_interval_label().is_empty());
            assert!(!lang.settings_order_label().is_empty());
            assert!(!lang.settings_order_random().is_empty());
            assert!(!lang.settings_order_seq().is_empty());
            assert!(!lang.settings_hwdec_title().is_empty());
            assert!(!lang.settings_hwdec_desc().is_empty());
            assert!(!lang.settings_battery_title().is_empty());
            assert!(!lang.settings_battery_desc().is_empty());
            assert!(!lang.status_battery_paused().is_empty());
            assert!(!lang.status_battery_resumed().is_empty());
            assert!(!lang.bar_volume().is_empty());
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
