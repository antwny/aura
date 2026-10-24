use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "es")]
    Es,
    #[serde(rename = "en")]
    En,
    #[serde(rename = "fr")]
    Fr,
}

impl Default for Language {
    fn default() -> Self {
        detect_system_language()
    }
}

pub fn detect_system_language() -> Language {
    if let Ok(lang) = std::env::var("LANG").or_else(|_| std::env::var("LC_MESSAGES")) {
        let lang = lang.to_lowercase();
        if lang.starts_with("es") {
            return Language::Es;
        }
        if lang.starts_with("fr") {
            return Language::Fr;
        }
    }
    Language::En
}

impl Language {
    pub fn from_code(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "es" | "spanish" | "español" => Language::Es,
            "fr" | "french" | "français" | "francais" => Language::Fr,
            _ => Language::En,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Language::Es => "es",
            Language::En => "en",
            Language::Fr => "fr",
        }
    }

    // --- Navigation ---
    pub fn nav_library(&self) -> &'static str {
        match self {
            Language::Es => "Biblioteca",
            Language::En => "Library",
            Language::Fr => "Bibliothèque",
        }
    }

    pub fn nav_explore(&self) -> &'static str {
        match self {
            Language::Es => "Explorar",
            Language::En => "Explore",
            Language::Fr => "Explorer",
        }
    }

    pub fn nav_monitors(&self) -> &'static str {
        match self {
            Language::Es => "Pantallas",
            Language::En => "Displays",
            Language::Fr => "Écrans",
        }
    }

    pub fn nav_settings(&self) -> &'static str {
        match self {
            Language::Es => "Ajustes",
            Language::En => "Settings",
            Language::Fr => "Paramètres",
        }
    }

    pub fn nav_about(&self) -> &'static str {
        match self {
            Language::Es => "Acerca de",
            Language::En => "About",
            Language::Fr => "À propos",
        }
    }

    // --- Header ---
    pub fn header_wallpapers_count(&self, count: usize) -> String {
        match self {
            Language::Es => format!("({} fondos)", count),
            Language::En => format!("({} wallpapers)", count),
            Language::Fr => format!("({} fonds d’écran)", count),
        }
    }

    pub fn header_add_video(&self) -> &'static str {
        match self {
            Language::Es => "Añadir Video",
            Language::En => "Add Video",
            Language::Fr => "Ajouter une vidéo",
        }
    }

    pub fn header_add_folder(&self) -> &'static str {
        match self {
            Language::Es => "Añadir Carpeta",
            Language::En => "Add Folder",
            Language::Fr => "Ajouter un dossier",
        }
    }

    // --- Library View ---
    pub fn library_search_placeholder(&self) -> &'static str {
        match self {
            Language::Es => "Buscar fondos por nombre...",
            Language::En => "Search wallpapers by name...",
            Language::Fr => "Rechercher des fonds d’écran par nom...",
        }
    }

    pub fn library_empty_title(&self) -> &'static str {
        match self {
            Language::Es => "Tu Biblioteca de Fondos está vacía",
            Language::En => "Your Wallpaper Library is empty",
            Language::Fr => "Votre bibliothèque de fonds d’écran est vide",
        }
    }

    pub fn library_empty_desc(&self) -> &'static str {
        match self {
            Language::Es => "Añade videos o carpetas para empezar a personalizar tu escritorio:",
            Language::En => "Add videos or folders to start customizing your desktop:",
            Language::Fr => {
                "Ajoutez des vidéos ou des dossiers pour commencer à personnaliser votre bureau :"
            }
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
            Language::Fr => "Extraction d’une image...",
        }
    }

    pub fn library_active(&self) -> &'static str {
        match self {
            Language::Es => "En uso",
            Language::En => "Active",
            Language::Fr => "Actif",
        }
    }

    pub fn library_apply(&self) -> &'static str {
        match self {
            Language::Es => "Aplicar",
            Language::En => "Apply",
            Language::Fr => "Appliquer",
        }
    }

    pub fn library_filter_all(&self) -> &'static str {
        match self {
            Language::Es => "Todos",
            Language::En => "All",
            Language::Fr => "Tous",
        }
    }

    pub fn library_filter_live(&self) -> &'static str {
        match self {
            Language::Es => "Animados",
            Language::En => "Live",
            Language::Fr => "Animés",
        }
    }

    pub fn library_filter_static(&self) -> &'static str {
        match self {
            Language::Es => "Estáticos",
            Language::En => "Static",
            Language::Fr => "Statiques",
        }
    }

    pub fn library_filter_downloaded(&self) -> &'static str {
        match self {
            Language::Es => "Descargados",
            Language::En => "Downloaded",
            Language::Fr => "Téléchargés",
        }
    }

    pub fn library_filter_favorites(&self) -> &'static str {
        match self {
            Language::Es => "Favoritos",
            Language::En => "Favorites",
            Language::Fr => "Favoris",
        }
    }

    pub fn library_favorites_empty(&self) -> &'static str {
        match self {
            Language::Es => "No tienes fondos favoritos aún",
            Language::En => "No favorite wallpapers yet",
            Language::Fr => "Vous n’avez encore aucun fond d’écran favori",
        }
    }

    pub fn library_favorites_empty_desc(&self) -> &'static str {
        match self {
            Language::Es => "Haz clic en el corazón de cualquier fondo para guardarlo aquí",
            Language::En => "Click the heart icon on any wallpaper to add it here",
            Language::Fr => {
                "Cliquez sur l’icône en forme de cœur d’un fond d’écran pour l’ajouter ici"
            }
        }
    }

    pub fn library_fav_add(&self) -> &'static str {
        match self {
            Language::Es => "Añadir a favoritos",
            Language::En => "Add to favorites",
            Language::Fr => "Ajouter aux favoris",
        }
    }

    pub fn library_fav_remove(&self) -> &'static str {
        match self {
            Language::Es => "Quitar de favoritos",
            Language::En => "Remove from favorites",
            Language::Fr => "Retirer des favoris",
        }
    }

    pub fn library_sort_newest(&self) -> &'static str {
        match self {
            Language::Es => "Más recientes",
            Language::En => "Newest",
            Language::Fr => "Plus récents",
        }
    }

    pub fn library_sort_oldest(&self) -> &'static str {
        match self {
            Language::Es => "Más antiguos",
            Language::En => "Oldest",
            Language::Fr => "Plus anciens",
        }
    }

    pub fn library_filter_empty(&self) -> &'static str {
        match self {
            Language::Es => "No se encontraron fondos en esta categoría o búsqueda",
            Language::En => "No wallpapers found in this category or search",
            Language::Fr => {
                "Aucun fond d’écran trouvé dans cette catégorie ou pour cette recherche"
            }
        }
    }

    #[allow(dead_code)]
    pub fn library_btn_delete(&self) -> &'static str {
        match self {
            Language::Es => "Eliminar",
            Language::En => "Delete",
            Language::Fr => "Supprimer",
        }
    }

    pub fn library_deleted_toast(&self) -> &'static str {
        match self {
            Language::Es => "Fondo eliminado de la biblioteca",
            Language::En => "Wallpaper removed from library",
            Language::Fr => "Fond d’écran supprimé de la bibliothèque",
        }
    }

    // --- Monitors View ---
    pub fn monitors_title(&self) -> &'static str {
        match self {
            Language::Es => "Visualizador Interactivo de Pantallas",
            Language::En => "Interactive Display Visualizer",
            Language::Fr => "Visualiseur interactif des écrans",
        }
    }

    pub fn monitors_desc(&self) -> &'static str {
        match self {
            Language::Es => "Representación a escala de tus pantallas detectadas en COSMIC. Configura fondos y escalado independiente:",
            Language::En => "True-to-scale layout of detected COSMIC displays. Configure wallpapers and scaling independently:",
            Language::Fr => "Disposition à l’échelle des écrans détectés dans COSMIC. Configurez indépendamment les fonds d’écran et leur mise à l’échelle :",
        }
    }

    pub fn monitors_screen_name(&self, name: &str) -> String {
        match self {
            Language::Es => format!("Pantalla {}", name),
            Language::En => format!("Display {}", name),
            Language::Fr => format!("Écran {}", name),
        }
    }

    pub fn monitors_stop(&self) -> &'static str {
        match self {
            Language::Es => "Detener",
            Language::En => "Stop",
            Language::Fr => "Arrêter",
        }
    }

    // --- Bottom Bar ("Now Playing") ---
    pub fn bar_idle_title(&self) -> &'static str {
        match self {
            Language::Es => "Ningún fondo en reproducción",
            Language::En => "No wallpaper playing",
            Language::Fr => "Aucun fond d’écran en cours",
        }
    }

    pub fn bar_idle_desc(&self) -> &'static str {
        match self {
            Language::Es => "Escritorio COSMIC estándar",
            Language::En => "Standard COSMIC desktop",
            Language::Fr => "Bureau COSMIC standard",
        }
    }

    pub fn bar_active_title_default(&self) -> &'static str {
        match self {
            Language::Es => "Fondo Activo",
            Language::En => "Active Wallpaper",
            Language::Fr => "Fond d’écran actif",
        }
    }

    pub fn bar_monitors_count(&self, count: usize) -> String {
        match self {
            Language::Es => format!("{} monitor{}", count, if count == 1 { "" } else { "es" }),
            Language::En => format!("{} monitor{}", count, if count == 1 { "" } else { "s" }),
            Language::Fr => format!("{} écran{}", count, if count == 1 { "" } else { "s" }),
        }
    }

    pub fn bar_all_monitors(&self) -> &'static str {
        match self {
            Language::Es => "Todos los monitores",
            Language::En => "All monitors",
            Language::Fr => "Tous les écrans",
        }
    }

    pub fn bar_this_monitor(&self) -> &'static str {
        match self {
            Language::Es => "Solo este monitor",
            Language::En => "This monitor only",
            Language::Fr => "Cet écran uniquement",
        }
    }

    pub fn bar_details(&self) -> &'static str {
        match self {
            Language::Es => "Detalles y opciones",
            Language::En => "Details and options",
            Language::Fr => "Détails et options",
        }
    }

    pub fn bar_target_label(&self) -> &'static str {
        match self {
            Language::Es => "Destino",
            Language::En => "Target",
            Language::Fr => "Cible",
        }
    }

    pub fn bar_display_label(&self, outputs: &str) -> String {
        match self {
            Language::Es => format!("Pantalla: {}", outputs),
            Language::En => format!("Display: {}", outputs),
            Language::Fr => format!("Écran : {}", outputs),
        }
    }

    pub fn bar_resume(&self) -> &'static str {
        match self {
            Language::Es => "Reanudar",
            Language::En => "Resume",
            Language::Fr => "Reprendre",
        }
    }

    pub fn bar_pause(&self) -> &'static str {
        match self {
            Language::Es => "Pausar",
            Language::En => "Pause",
            Language::Fr => "Mettre en pause",
        }
    }

    pub fn bar_prev(&self) -> &'static str {
        match self {
            Language::Es => "Fondo anterior",
            Language::En => "Previous wallpaper",
            Language::Fr => "Fond d’écran précédent",
        }
    }

    pub fn bar_next(&self) -> &'static str {
        match self {
            Language::Es => "Siguiente fondo",
            Language::En => "Next wallpaper",
            Language::Fr => "Fond d’écran suivant",
        }
    }

    pub fn bar_muted(&self) -> &'static str {
        match self {
            Language::Es => "Silenciado",
            Language::En => "Muted",
            Language::Fr => "Muet",
        }
    }

    pub fn bar_audio_active(&self) -> &'static str {
        match self {
            Language::Es => "Sonido Activo",
            Language::En => "Audio On",
            Language::Fr => "Son activé",
        }
    }

    pub fn bar_stop(&self) -> &'static str {
        match self {
            Language::Es => "Detener",
            Language::En => "Stop",
            Language::Fr => "Arrêter",
        }
    }

    pub fn bar_gpu_accel(&self, hwdec: &str) -> String {
        match self {
            Language::Es => format!("Aceleración GPU: {}", hwdec),
            Language::En => format!("GPU Acceleration: {}", hwdec),
            Language::Fr => format!("Accélération GPU : {}", hwdec),
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
            Language::Fr => "Paramètres système",
        }
    }

    pub fn settings_lang_title(&self) -> &'static str {
        match self {
            Language::Es => "Idioma",
            Language::En => "Language",
            Language::Fr => "Langue",
        }
    }

    pub fn settings_lang_desc(&self) -> &'static str {
        match self {
            Language::Es => "Selecciona el idioma de la aplicación. El cambio se aplica inmediatamente sin reiniciar.",
            Language::En => "Select the application language. Changes take effect instantly without restarting.",
            Language::Fr => "Sélectionnez la langue de l’application. Le changement s’applique immédiatement, sans redémarrage.",
        }
    }

    pub fn settings_section_general(&self) -> &'static str {
        match self {
            Language::Es => "General y Sistema",
            Language::En => "General & System",
            Language::Fr => "Général et système",
        }
    }

    pub fn settings_section_engine(&self) -> &'static str {
        match self {
            Language::Es => "Entorno y Motor",
            Language::En => "Environment & Engine",
            Language::Fr => "Environnement et moteur",
        }
    }

    pub fn settings_section_performance(&self) -> &'static str {
        match self {
            Language::Es => "Rendimiento y Energía",
            Language::En => "Performance & Energy",
            Language::Fr => "Performances et énergie",
        }
    }

    pub fn settings_section_appearance(&self) -> &'static str {
        match self {
            Language::Es => "Apariencia y Colores",
            Language::En => "Appearance & Colors",
            Language::Fr => "Apparence et couleurs",
        }
    }

    pub fn settings_engine_desktop_lbl(&self) -> &'static str {
        match self {
            Language::Es => "Entorno detectado:",
            Language::En => "Detected environment:",
            Language::Fr => "Environnement détecté :",
        }
    }

    pub fn settings_engine_backend_lbl(&self) -> &'static str {
        match self {
            Language::Es => "Motor activo:",
            Language::En => "Active engine:",
            Language::Fr => "Moteur actif :",
        }
    }

    pub fn settings_engine_hanabi_status_ok(&self) -> &'static str {
        match self {
            Language::Es => "Extensión Hanabi detectada y operativa para fondos animados.",
            Language::En => "Hanabi extension detected and ready for animated wallpapers.",
            Language::Fr => {
                "L’extension Hanabi est détectée et prête pour les fonds d’écran animés."
            }
        }
    }

    pub fn settings_engine_hanabi_status_missing(&self) -> &'static str {
        match self {
            Language::Es => "Se requiere la extensión Hanabi para reproducir vídeos en GNOME.",
            Language::En => "Hanabi extension is required to play video wallpapers on GNOME.",
            Language::Fr => {
                "L’extension Hanabi est nécessaire pour lire des fonds d’écran vidéo sous GNOME."
            }
        }
    }

    pub fn settings_engine_hanabi_install_btn(&self) -> &'static str {
        match self {
            Language::Es => "Instalar Hanabi",
            Language::En => "Install Hanabi",
            Language::Fr => "Installer Hanabi",
        }
    }

    pub fn settings_cosmic_badge(&self) -> &'static str {
        match self {
            Language::Es => "COSMIC Desktop",
            Language::En => "COSMIC Desktop",
            Language::Fr => "COSMIC Desktop",
        }
    }

    pub fn settings_autostart_desktop(&self, is_gnome: bool) -> &'static str {
        if is_gnome {
            match self {
                Language::Es => "Iniciar fondo animado automáticamente al iniciar sesión en GNOME",
                Language::En => "Launch animated wallpaper automatically upon login to GNOME",
                Language::Fr => {
                    "Lancer automatiquement le fond d’écran animé à l’ouverture de session GNOME"
                }
            }
        } else {
            self.settings_autostart()
        }
    }

    pub fn settings_autostart(&self) -> &'static str {
        match self {
            Language::Es => "Iniciar fondo animado automáticamente al iniciar sesión en COSMIC Desktop",
            Language::En => "Launch animated wallpaper automatically upon login to COSMIC Desktop",
            Language::Fr => "Lancer automatiquement le fond d’écran animé à l’ouverture de session COSMIC Desktop",
        }
    }

    pub fn settings_smart_pause(&self) -> &'static str {
        match self {
            Language::Es => "Pausar automáticamente en juegos y ventanas a pantalla completa (Smart Pause)",
            Language::En => "Automatically pause in fullscreen games and windows (Smart Pause)",
            Language::Fr => "Mettre automatiquement en pause dans les jeux et fenêtres en plein écran (pause intelligente)",
        }
    }

    pub fn settings_auto_pause(&self) -> &'static str {
        match self {
            Language::Es => "Pausar automáticamente si el escritorio está oculto",
            Language::En => "Automatically pause when desktop is hidden",
            Language::Fr => "Mettre automatiquement en pause lorsque le bureau est masqué",
        }
    }

    pub fn settings_auto_pause_desc(&self) -> &'static str {
        match self {
            Language::Es => "Detiene la reproducción para 0% de uso de GPU/CPU cuando las ventanas cubren el fondo",
            Language::En => "Stops playback for 0% GPU/CPU usage when windows cover the wallpaper",
            Language::Fr => "Arrête la lecture pour réduire l’utilisation GPU/CPU à 0 % lorsque des fenêtres couvrent le fond d’écran",
        }
    }

    pub fn settings_keep_running(&self) -> &'static str {
        match self {
            Language::Es => "Minimizar a la barra superior (bandeja del sistema) al cerrar la ventana",
            Language::En => "Minimize to the top bar (system tray) when closing window",
            Language::Fr => "Réduire dans la barre supérieure (zone de notification) à la fermeture de la fenêtre",
        }
    }

    pub fn settings_auto_theme(&self) -> &'static str {
        match self {
            Language::Es => "Sincronizar color de acento del sistema con el fondo (Auto-Tema COSMIC)",
            Language::En => "Synchronize system accent color with active wallpaper (COSMIC Auto-Theme)",
            Language::Fr => "Synchroniser la couleur d’accentuation du système avec le fond d’écran actif (thème automatique COSMIC)",
        }
    }

    pub fn settings_auto_theme_desc(&self) -> &'static str {
        match self {
            Language::Es => "Extrae el color predominante del fondo y lo aplica como acento del sistema en COSMIC",
            Language::En => "Extracts dominant wallpaper color and sets it as the COSMIC system accent",
            Language::Fr => "Extrait la couleur dominante du fond d’écran et l’utilise comme couleur d’accentuation de COSMIC",
        }
    }

    pub fn settings_theme_mode_title(&self) -> &'static str {
        match self {
            Language::Es => "Modo de Apariencia del Sistema (COSMIC)",
            Language::En => "System Appearance Mode (COSMIC)",
            Language::Fr => "Mode d’apparence du système (COSMIC)",
        }
    }

    pub fn settings_theme_mode_auto(&self) -> &'static str {
        match self {
            Language::Es => "Automático",
            Language::En => "Dynamic",
            Language::Fr => "Dynamique",
        }
    }

    pub fn settings_theme_mode_auto_desc(&self) -> &'static str {
        match self {
            Language::Es => "Cambia dinámicamente entre Modo Oscuro y Claro según la claridad del fondo activo.",
            Language::En => "Dynamically toggles Dark and Light mode based on active wallpaper luminance.",
            Language::Fr => "Bascule dynamiquement entre les modes sombre et clair selon la luminosité du fond d’écran actif.",
        }
    }

    pub fn settings_theme_mode_dark(&self) -> &'static str {
        match self {
            Language::Es => "Oscuro fijo",
            Language::En => "Fixed Dark",
            Language::Fr => "Sombre fixe",
        }
    }

    pub fn settings_theme_mode_dark_desc(&self) -> &'static str {
        match self {
            Language::Es => "El sistema permanece siempre en Modo Oscuro. Se mantiene fijo sin importar qué fondo apliques o cómo rote la lista.",
            Language::En => "System permanently stays in Dark Mode. Persists regardless of wallpaper changes or playlist rotation.",
            Language::Fr => "Le système reste en permanence en mode sombre, quels que soient les changements de fond d’écran ou la rotation de la liste.",
        }
    }

    pub fn settings_theme_mode_light(&self) -> &'static str {
        match self {
            Language::Es => "Claro fijo",
            Language::En => "Fixed Light",
            Language::Fr => "Clair fixe",
        }
    }

    pub fn settings_theme_mode_light_desc(&self) -> &'static str {
        match self {
            Language::Es => "El sistema permanece siempre en Modo Claro. Se mantiene fijo sin importar qué fondo apliques o cómo rote la lista.",
            Language::En => "System permanently stays in Light Mode. Persists regardless of wallpaper changes or playlist rotation.",
            Language::Fr => "Le système reste en permanence en mode clair, quels que soient les changements de fond d’écran ou la rotation de la liste.",
        }
    }

    pub fn settings_theme_mode_manual(&self) -> &'static str {
        match self {
            Language::Es => "Sin alterar",
            Language::En => "System Default",
            Language::Fr => "Par défaut du système",
        }
    }

    pub fn settings_theme_mode_manual_desc(&self) -> &'static str {
        match self {
            Language::Es => "Aura no modificará el modo oscuro/claro del sistema, respetando tu configuración manual o programada de COSMIC.",
            Language::En => "Aura will not alter the system dark/light mode, respecting your manual or scheduled COSMIC settings.",
            Language::Fr => "Aura ne modifiera pas le mode clair/sombre du système et respectera vos réglages COSMIC manuels ou programmés.",
        }
    }

    pub fn status_theme_mode_dark(&self) -> &'static str {
        match self {
            Language::Es => "✨ Modo Oscuro fijo aplicado y persistente",
            Language::En => "✨ Fixed Dark Mode applied and persistent",
            Language::Fr => "✨ Mode sombre fixe appliqué et persistant",
        }
    }

    pub fn status_theme_mode_light(&self) -> &'static str {
        match self {
            Language::Es => "✨ Modo Claro fijo aplicado y persistente",
            Language::En => "✨ Fixed Light Mode applied and persistent",
            Language::Fr => "✨ Mode clair fixe appliqué et persistant",
        }
    }

    pub fn status_theme_mode_auto(&self) -> &'static str {
        match self {
            Language::Es => "✨ Modo Automático activado (sincronizado con fondo)",
            Language::En => "✨ Dynamic Mode activated (synced with wallpaper)",
            Language::Fr => "✨ Mode dynamique activé (synchronisé avec le fond d’écran)",
        }
    }

    pub fn status_theme_mode_manual(&self) -> &'static str {
        match self {
            Language::Es => "✨ Modo del sistema sin alterar por Aura",
            Language::En => "✨ System theme mode left unaltered by Aura",
            Language::Fr => "✨ Mode du thème système laissé inchangé par Aura",
        }
    }

    #[allow(dead_code)]
    pub fn settings_auto_dark(&self) -> &'static str {
        match self {
            Language::Es => "Cambiar automáticamente entre Modo Oscuro y Claro según la claridad del video",
            Language::En => "Automatically toggle Dark and Light mode based on video luminance",
            Language::Fr => "Basculer automatiquement entre les modes sombre et clair selon la luminosité de la vidéo",
        }
    }

    pub fn settings_monitored_folders(&self) -> &'static str {
        match self {
            Language::Es => "Carpetas Monitoreadas",
            Language::En => "Monitored Folders",
            Language::Fr => "Dossiers surveillés",
        }
    }

    pub fn settings_btn_add_folder(&self) -> &'static str {
        match self {
            Language::Es => "Añadir Carpeta",
            Language::En => "Add Folder",
            Language::Fr => "Ajouter un dossier",
        }
    }

    pub fn settings_btn_delete(&self) -> &'static str {
        match self {
            Language::Es => "Eliminar",
            Language::En => "Remove",
            Language::Fr => "Retirer",
        }
    }

    #[allow(dead_code)]
    pub fn settings_persistence_title(&self) -> &'static str {
        match self {
            Language::Es => "Persistencia en segundo plano",
            Language::En => "Background Persistence",
            Language::Fr => "Persistance en arrière-plan",
        }
    }

    #[allow(dead_code)]
    pub fn settings_persistence_desc(&self) -> &'static str {
        match self {
            Language::Es => "Aura funciona como centro de control. Al cerrar esta ventana, tu fondo animado continuará reproduciéndose sin problemas en tu compositor Wayland a través de mpvpaper, liberando el 100% de la memoria de la interfaz.",
            Language::En => "Aura serves as a control center. When this window is closed, your live wallpaper keeps playing seamlessly in your Wayland compositor via mpvpaper, freeing 100% of the UI memory.",
            Language::Fr => "Aura sert de centre de contrôle. Lorsque cette fenêtre est fermée, votre fond d’écran animé continue de fonctionner dans votre compositeur Wayland via mpvpaper, tout en libérant 100 % de la mémoire utilisée par l’interface.",
        }
    }

    // --- About View ---
    pub fn about_tagline(&self) -> &'static str {
        if crate::engine::gnome::is_active() {
            match self {
                Language::Es => "Gestor de Fondos Animados para GNOME y Wayland",
                Language::En => "Live Wallpaper Manager for GNOME & Wayland",
                Language::Fr => "Gestionnaire de fonds d’écran animés pour GNOME et Wayland",
            }
        } else {
            match self {
                Language::Es => "Gestor Nativo de Fondos Animados para COSMIC Desktop",
                Language::En => "Native Live Wallpaper Manager for COSMIC Desktop",
                Language::Fr => "Gestionnaire natif de fonds d’écran animés pour COSMIC Desktop",
            }
        }
    }

    pub fn about_version_info(&self) -> String {
        let os = crate::engine::mpvpaper::detect_os_pretty_name();
        let desktop = if crate::engine::gnome::is_active() {
            "GNOME Shell"
        } else {
            "COSMIC Desktop"
        };
        match self {
            Language::Es => format!(
                "Versión {} • {} • {}",
                env!("CARGO_PKG_VERSION"),
                os,
                desktop
            ),
            Language::En => format!(
                "Version {} • {} • {}",
                env!("CARGO_PKG_VERSION"),
                os,
                desktop
            ),
            Language::Fr => format!(
                "Version {} • {} • {}",
                env!("CARGO_PKG_VERSION"),
                os,
                desktop
            ),
        }
    }

    #[allow(dead_code)]
    pub fn about_details_title(&self) -> &'static str {
        match self {
            Language::Es => "Detalles de la Aplicación",
            Language::En => "Application Details",
            Language::Fr => "Détails de l’application",
        }
    }

    pub fn about_developer_lbl(&self) -> &'static str {
        match self {
            Language::Es => "Desarrollador:",
            Language::En => "Developer:",
            Language::Fr => "Développeur :",
        }
    }

    #[allow(dead_code)]
    pub fn about_architecture_lbl(&self) -> &'static str {
        match self {
            Language::Es => "Arquitectura:",
            Language::En => "Architecture:",
            Language::Fr => "Architecture :",
        }
    }

    pub fn about_license_lbl(&self) -> &'static str {
        match self {
            Language::Es => "Licencia:",
            Language::En => "License:",
            Language::Fr => "Licence :",
        }
    }

    #[allow(dead_code)]
    pub fn about_summary_desc(&self) -> &'static str {
        match self {
            Language::Es => "Aura fue concebido para transformar la experiencia de fondos de pantalla animados en Linux. Aprovechando el poder nativo de Rust y libcosmic, ofrece un rendimiento fluido de 60 FPS con huella de memoria ultraligera, decodificación completa por GPU y sincronización automática de color.",
            Language::En => "Aura was engineered to redefine animated live wallpapers on Linux. Harnessing native Rust and libcosmic, it delivers smooth 60 FPS playback with minimal memory footprint, full GPU hardware acceleration, and dynamic cosmic color synchronization.",
            Language::Fr => "Aura a été conçu pour repenser les fonds d’écran animés sous Linux. Grâce à Rust et libcosmic, il offre une lecture fluide à 60 FPS, une faible empreinte mémoire, une accélération matérielle GPU complète et une synchronisation dynamique des couleurs de COSMIC.",
        }
    }

    pub fn about_github_btn(&self) -> &'static str {
        match self {
            Language::Es => "Github",
            Language::En => "Github",
            Language::Fr => "GitHub",
        }
    }

    pub fn about_youtube_btn(&self) -> &'static str {
        match self {
            Language::Es => "YouTube",
            Language::En => "YouTube",
            Language::Fr => "YouTube",
        }
    }

    #[allow(dead_code)]
    pub fn about_donation_lbl(&self) -> &'static str {
        match self {
            Language::Es => "Donaciones:",
            Language::En => "Donations:",
            Language::Fr => "Dons :",
        }
    }

    pub fn about_donate_btn(&self) -> &'static str {
        match self {
            Language::Es => "Donar (PayPal)",
            Language::En => "Donate (PayPal)",
            Language::Fr => "Faire un don (PayPal)",
        }
    }

    pub fn about_updates_title(&self) -> &'static str {
        match self {
            Language::Es => "Actualizaciones de Software",
            Language::En => "Software Updates",
            Language::Fr => "Mises à jour logicielles",
        }
    }

    pub fn about_check_updates_btn(&self) -> &'static str {
        match self {
            Language::Es => "Buscar actualizaciones",
            Language::En => "Check for updates",
            Language::Fr => "Rechercher des mises à jour",
        }
    }

    pub fn about_checking_updates(&self) -> &'static str {
        match self {
            Language::Es => "Buscando actualizaciones en GitHub...",
            Language::En => "Checking for updates on GitHub...",
            Language::Fr => "Recherche de mises à jour sur GitHub...",
        }
    }

    pub fn about_up_to_date(&self) -> &'static str {
        match self {
            Language::Es => "Aura está al día con la última versión.",
            Language::En => "Aura is up to date.",
            Language::Fr => "Aura est à jour.",
        }
    }

    pub fn about_update_available(&self) -> &'static str {
        match self {
            Language::Es => "Nueva versión disponible:",
            Language::En => "New version available:",
            Language::Fr => "Nouvelle version disponible :",
        }
    }

    pub fn about_update_now_btn(&self) -> &'static str {
        match self {
            Language::Es => "Actualizar a",
            Language::En => "Update to",
            Language::Fr => "Mettre à jour vers",
        }
    }

    pub fn about_updating(&self) -> &'static str {
        match self {
            Language::Es => "Descargando e instalando actualización...",
            Language::En => "Downloading and installing update...",
            Language::Fr => "Téléchargement et installation de la mise à jour...",
        }
    }

    pub fn about_restart_btn(&self) -> &'static str {
        match self {
            Language::Es => "Reiniciar Aura",
            Language::En => "Restart Aura",
            Language::Fr => "Redémarrer Aura",
        }
    }

    pub fn about_flatpak_managed(&self) -> &'static str {
        match self {
            Language::Es => "Versión Flatpak: las actualizaciones se gestionan automáticamente a través de COSMIC App Store o Flathub.",
            Language::En => "Flatpak edition: updates are managed via COSMIC App Store or Flathub.",
            Language::Fr => "Édition Flatpak : les mises à jour sont gérées via le COSMIC App Store ou Flathub.",
        }
    }

    pub fn toast_update_available_action(&self) -> &'static str {
        match self {
            Language::Es => "Actualizar",
            Language::En => "Update",
            Language::Fr => "Mettre à jour",
        }
    }

    pub fn toast_restart_action(&self) -> &'static str {
        match self {
            Language::Es => "Reiniciar",
            Language::En => "Restart",
            Language::Fr => "Redémarrer",
        }
    }

    // --- Explore View (Online Catalog) ---
    pub fn explore_source_motionbgs(&self) -> &'static str {
        match self {
            Language::Es => "Live Wallpapers",
            Language::En => "Live Wallpapers",
            Language::Fr => "Fonds d’écran animés",
        }
    }

    pub fn explore_source_bing(&self) -> &'static str {
        match self {
            Language::Es => "Bing del Día (4K)",
            Language::En => "Bing Daily (4K)",
            Language::Fr => "Bing du jour (4K)",
        }
    }

    pub fn explore_source_wallhaven(&self) -> &'static str {
        match self {
            Language::Es => "Wallhaven (Top 4K)",
            Language::En => "Wallhaven (Top 4K)",
            Language::Fr => "Wallhaven (Top 4K)",
        }
    }

    pub fn explore_source_minimalistic(&self) -> &'static str {
        match self {
            Language::Es => "Minimalista",
            Language::En => "Minimalist",
            Language::Fr => "Minimaliste",
        }
    }

    pub fn explore_motionbgs_subtitle(&self) -> &'static str {
        match self {
            Language::Es => "Catálogo de 9,400+ fondos animados en bucle 4K y HD de MotionBGS",
            Language::En => "Catalog of 9,400+ 4K and HD looping animated wallpapers from MotionBGS",
            Language::Fr => "Catalogue de plus de 9 400 fonds d’écran animés 4K et HD en boucle provenant de MotionBGS",
        }
    }

    pub fn explore_search_motionbgs_placeholder(&self) -> &'static str {
        match self {
            Language::Es => "Buscar fondos animados (ej: anime, cyberpunk, espacio, rain)...",
            Language::En => "Search live wallpapers (e.g. anime, cyberpunk, space, rain)...",
            Language::Fr => {
                "Rechercher des fonds d’écran animés (ex. : anime, cyberpunk, espace, pluie)..."
            }
        }
    }

    pub fn explore_minimalistic_subtitle(&self) -> &'static str {
        match self {
            Language::Es => "Colección curada de 300+ fondos de naturaleza digital y arte plano",
            Language::En => "Curated collection of 300+ digital nature & flat art wallpapers",
            Language::Fr => "Collection sélectionnée de plus de 300 fonds d’écran de nature numérique et de flat design",
        }
    }

    pub fn explore_search_minimal_placeholder(&self) -> &'static str {
        match self {
            Language::Es => "Buscar por título o artista...",
            Language::En => "Search by title or artist...",
            Language::Fr => "Rechercher par titre ou artiste...",
        }
    }

    pub fn explore_btn_download(&self) -> &'static str {
        match self {
            Language::Es => "Descargar",
            Language::En => "Download",
            Language::Fr => "Télécharger",
        }
    }

    pub fn explore_btn_downloading(&self) -> &'static str {
        match self {
            Language::Es => "Descargando...",
            Language::En => "Downloading...",
            Language::Fr => "Téléchargement...",
        }
    }

    pub fn explore_btn_cancel(&self) -> &'static str {
        match self {
            Language::Es => "Cancelar",
            Language::En => "Cancel",
            Language::Fr => "Annuler",
        }
    }

    pub fn explore_btn_apply(&self) -> &'static str {
        match self {
            Language::Es => "Aplicar",
            Language::En => "Apply",
            Language::Fr => "Appliquer",
        }
    }

    pub fn explore_badge_active(&self) -> &'static str {
        match self {
            Language::Es => "En Uso",
            Language::En => "Active",
            Language::Fr => "Actif",
        }
    }

    pub fn explore_featured_today(&self) -> &'static str {
        match self {
            Language::Es => "Foto destacada de hoy en 4K UHD",
            Language::En => "Daily featured photo in 4K UHD",
            Language::Fr => "Photo du jour en 4K UHD",
        }
    }

    pub fn explore_recent_title(&self) -> &'static str {
        match self {
            Language::Es => "Galería y archivo 4K",
            Language::En => "4K gallery & archive",
            Language::Fr => "Galerie et archives 4K",
        }
    }

    pub fn explore_loading(&self) -> &'static str {
        match self {
            Language::Es => "Cargando catálogo en línea...",
            Language::En => "Loading online catalog...",
            Language::Fr => "Chargement du catalogue en ligne...",
        }
    }

    pub fn explore_error_prefix(&self) -> &'static str {
        match self {
            Language::Es => "No se pudo conectar con el catálogo en línea:",
            Language::En => "Could not connect to online catalog:",
            Language::Fr => "Impossible de se connecter au catalogue en ligne :",
        }
    }

    pub fn explore_btn_retry(&self) -> &'static str {
        match self {
            Language::Es => "Reintentar conexión",
            Language::En => "Retry connection",
            Language::Fr => "Réessayer",
        }
    }

    pub fn explore_toast_downloaded(&self) -> &'static str {
        match self {
            Language::Es => "Fondo descargado con éxito",
            Language::En => "Wallpaper downloaded successfully",
            Language::Fr => "Fond d’écran téléchargé avec succès",
        }
    }

    pub fn open_in_file_manager(&self) -> &'static str {
        match self {
            Language::Es => "Abrir carpeta",
            Language::En => "Open folder",
            Language::Fr => "Ouvrir le dossier",
        }
    }

    pub fn toast_show_in_files(&self) -> &'static str {
        match self {
            Language::Es => "Ver en archivos",
            Language::En => "Show in files",
            Language::Fr => "Afficher dans le gestionnaire de fichiers",
        }
    }

    pub fn toast_folder_opened(&self) -> &'static str {
        match self {
            Language::Es => "Carpeta de fondos abierta en el Gestor de Archivos",
            Language::En => "Wallpapers folder opened in File Manager",
            Language::Fr => "Dossier des fonds d’écran ouvert dans le gestionnaire de fichiers",
        }
    }

    pub fn toast_folder_open_failed(&self) -> &'static str {
        match self {
            Language::Es => "No se pudo abrir la carpeta en el Gestor de Archivos",
            Language::En => "Could not open folder in File Manager",
            Language::Fr => "Impossible d’ouvrir le dossier dans le gestionnaire de fichiers",
        }
    }

    // --- GNOME backend ---

    /// Shown as a toast when the user picks a video on GNOME but Hanabi is not installed.
    pub fn gnome_hanabi_required_toast(&self) -> &'static str {
        match self {
            Language::Es => "Los vídeos en GNOME requieren la extensión Hanabi",
            Language::En => "Videos on GNOME require the Hanabi extension",
            Language::Fr => "Les vidéos sous GNOME nécessitent l’extension Hanabi",
        }
    }

    /// Action label on the Hanabi toast that opens the extension page.
    pub fn gnome_hanabi_install_action(&self) -> &'static str {
        match self {
            Language::Es => "Instalar Hanabi",
            Language::En => "Install Hanabi",
            Language::Fr => "Installer Hanabi",
        }
    }

    /// Full error message when Hanabi is missing (used in engine error path).
    pub fn gnome_hanabi_missing_error(&self) -> &'static str {
        match self {
            Language::Es => "GNOME no permite fondos de vídeo sin la extensión Hanabi. Instala gnome-ext-hanabi para usar vídeos.",
            Language::En => "GNOME does not support video wallpapers without the Hanabi extension. Install gnome-ext-hanabi to use videos.",
            Language::Fr => "GNOME ne prend pas en charge les fonds d’écran vidéo sans l’extension Hanabi. Installez gnome-ext-hanabi pour utiliser des vidéos.",
        }
    }

    /// Shown when Hanabi is installed but GNOME has not loaded it yet.
    pub fn gnome_hanabi_not_loaded(&self) -> &'static str {
        match self {
            Language::Es => "Hanabi está instalado pero GNOME no lo ha cargado. Cierra sesión y vuelve a entrar.",
            Language::En => "Hanabi is installed but GNOME has not loaded it yet. Log out and back in.",
            Language::Fr => "Hanabi est installé, mais GNOME ne l’a pas encore chargé. Déconnectez-vous puis reconnectez-vous.",
        }
    }

    /// Shown when Hanabi cannot be disabled while switching to an image wallpaper.
    pub fn gnome_hanabi_disable_failed(&self) -> &'static str {
        match self {
            Language::Es => "No se pudo desactivar Hanabi.",
            Language::En => "Could not disable Hanabi.",
            Language::Fr => "Impossible de désactiver Hanabi.",
        }
    }

    /// Shown when Hanabi accepts settings but its renderer does not start playback.
    pub fn gnome_hanabi_playback_failed(&self) -> &'static str {
        match self {
            Language::Es => "Hanabi no inició el fondo animado. Comprueba que el vídeo se pueda reproducir y que Hanabi esté activo.",
            Language::En => "Hanabi did not start the animated wallpaper. Check that the video is playable and Hanabi is enabled.",
            Language::Fr => "Hanabi n’a pas lancé le fond d’écran animé. Vérifiez que la vidéo est lisible et que Hanabi est activé.",
        }
    }

    pub fn explore_btn_load_more(&self) -> &'static str {
        match self {
            Language::Es => "Cargar más fondos",
            Language::En => "Load more wallpapers",
            Language::Fr => "Charger plus de fonds d’écran",
        }
    }

    pub fn library_btn_load_more(&self) -> &'static str {
        match self {
            Language::Es => "Cargar más fondos",
            Language::En => "Load more wallpapers",
            Language::Fr => "Charger plus de fonds d’écran",
        }
    }

    pub fn explore_btn_loading_more(&self) -> &'static str {
        match self {
            Language::Es => "Cargando más fondos...",
            Language::En => "Loading more wallpapers...",
            Language::Fr => "Chargement de fonds d’écran supplémentaires...",
        }
    }

    pub fn explore_search_placeholder(&self) -> &'static str {
        match self {
            Language::Es => "Buscar en Wallhaven (ej: anime, cyberpunk, nature)...",
            Language::En => "Search Wallhaven (e.g. anime, cyberpunk, nature)...",
            Language::Fr => "Rechercher sur Wallhaven (ex. : anime, cyberpunk, nature)...",
        }
    }

    pub fn explore_cat_all(&self) -> &'static str {
        match self {
            Language::Es => "Todos",
            Language::En => "All",
            Language::Fr => "Tous",
        }
    }

    pub fn explore_cat_anime(&self) -> &'static str {
        match self {
            Language::Es => "Anime",
            Language::En => "Anime",
            Language::Fr => "Anime",
        }
    }

    pub fn explore_cat_general(&self) -> &'static str {
        match self {
            Language::Es => "Naturaleza / General",
            Language::En => "Nature / General",
            Language::Fr => "Nature / Général",
        }
    }

    pub fn explore_cat_games(&self) -> &'static str {
        match self {
            Language::Es => "Videojuegos",
            Language::En => "Games",
            Language::Fr => "Jeux",
        }
    }

    pub fn explore_cat_nature(&self) -> &'static str {
        match self {
            Language::Es => "Naturaleza",
            Language::En => "Nature",
            Language::Fr => "Nature",
        }
    }

    pub fn explore_cat_space(&self) -> &'static str {
        match self {
            Language::Es => "Espacio / Sci-Fi",
            Language::En => "Space / Sci-Fi",
            Language::Fr => "Espace / Science-fiction",
        }
    }

    pub fn explore_cat_cars(&self) -> &'static str {
        match self {
            Language::Es => "Autos",
            Language::En => "Cars",
            Language::Fr => "Voitures",
        }
    }

    pub fn explore_cat_superhero(&self) -> &'static str {
        match self {
            Language::Es => "Superhéroes",
            Language::En => "Superheroes",
            Language::Fr => "Super-héros",
        }
    }

    pub fn explore_cat_fantasy(&self) -> &'static str {
        match self {
            Language::Es => "Fantasía",
            Language::En => "Fantasy",
            Language::Fr => "Fantastique",
        }
    }

    pub fn explore_cat_technology(&self) -> &'static str {
        match self {
            Language::Es => "Tecnología",
            Language::En => "Technology",
            Language::Fr => "Technologie",
        }
    }

    pub fn explore_sort_top(&self) -> &'static str {
        match self {
            Language::Es => "Más Votados",
            Language::En => "Top Rated",
            Language::Fr => "Les mieux notés",
        }
    }

    pub fn explore_sort_hot(&self) -> &'static str {
        match self {
            Language::Es => "Tendencias",
            Language::En => "Hot",
            Language::Fr => "Tendances",
        }
    }

    pub fn explore_sort_random(&self) -> &'static str {
        match self {
            Language::Es => "Aleatorio",
            Language::En => "Random",
            Language::Fr => "Aléatoire",
        }
    }

    pub fn explore_res_all(&self) -> &'static str {
        match self {
            Language::Es => "Resolución: Todas",
            Language::En => "Resolution: All",
            Language::Fr => "Résolution : toutes",
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

    pub fn explore_res_1080p(&self) -> &'static str {
        "1080p HD"
    }

    pub fn explore_resolution_label(&self) -> &'static str {
        match self {
            Language::Es => "Resolución:",
            Language::En => "Resolution:",
            Language::Fr => "Résolution :",
        }
    }

    // --- Tray (StatusNotifierItem) ---
    pub fn tray_switcher(&self) -> &'static str {
        match self {
            Language::Es => "Selector Rápido de Fondos...",
            Language::En => "Quick Wallpaper Switcher...",
            Language::Fr => "Sélecteur rapide de fonds d’écran...",
        }
    }

    pub fn tray_open(&self) -> &'static str {
        match self {
            Language::Es => "Abrir Aura",
            Language::En => "Open Aura",
            Language::Fr => "Ouvrir Aura",
        }
    }

    pub fn tray_pause(&self) -> &'static str {
        match self {
            Language::Es => "Pausar Fondo",
            Language::En => "Pause Wallpaper",
            Language::Fr => "Mettre le fond d’écran en pause",
        }
    }

    pub fn tray_resume(&self) -> &'static str {
        match self {
            Language::Es => "Reanudar Fondo",
            Language::En => "Resume Wallpaper",
            Language::Fr => "Reprendre le fond d’écran",
        }
    }

    pub fn tray_next(&self) -> &'static str {
        match self {
            Language::Es => "Siguiente Fondo",
            Language::En => "Next Wallpaper",
            Language::Fr => "Fond d’écran suivant",
        }
    }

    pub fn tray_stop(&self) -> &'static str {
        match self {
            Language::Es => "Detener Fondo",
            Language::En => "Stop Wallpaper",
            Language::Fr => "Arrêter le fond d’écran",
        }
    }

    pub fn tray_quit(&self) -> &'static str {
        match self {
            Language::Es => "Salir de Aura",
            Language::En => "Quit Aura",
            Language::Fr => "Quitter Aura",
        }
    }

    pub fn tray_tooltip_idle(&self) -> &'static str {
        match self {
            Language::Es => "Sin fondo en reproducción",
            Language::En => "No wallpaper playing",
            Language::Fr => "Aucun fond d’écran en cours",
        }
    }

    pub fn tray_tooltip_playing(&self, name: &str) -> String {
        match self {
            Language::Es => format!("Reproduciendo: {}", name),
            Language::En => format!("Playing: {}", name),
            Language::Fr => format!("Lecture : {}", name),
        }
    }

    // --- Status Messages / Toasts ---
    pub fn status_lang_changed(&self) -> &'static str {
        match self {
            Language::Es => "Idioma cambiado a Español",
            Language::En => "Language switched to English",
            Language::Fr => "Langue changée en français",
        }
    }

    pub fn status_applied(&self, output: &str, name: &str) -> String {
        match self {
            Language::Es => format!("Fondo aplicado en {}: '{}'", output, name),
            Language::En => format!("Wallpaper applied on {}: '{}'", output, name),
            Language::Fr => format!("Fond d’écran appliqué sur {} : '{}'", output, name),
        }
    }

    pub fn status_paused(&self) -> &'static str {
        match self {
            Language::Es => "Fondo pausado (0% GPU/CPU)",
            Language::En => "Wallpaper paused (0% GPU/CPU)",
            Language::Fr => "Fond d’écran en pause (0 % GPU/CPU)",
        }
    }

    pub fn status_resumed(&self) -> &'static str {
        match self {
            Language::Es => "Fondo reanudado",
            Language::En => "Wallpaper resumed",
            Language::Fr => "Lecture du fond d’écran reprise",
        }
    }

    pub fn status_stopped_output(&self, output: &str) -> String {
        match self {
            Language::Es => format!("Fondo detenido en {}", output),
            Language::En => format!("Wallpaper stopped on {}", output),
            Language::Fr => format!("Fond d’écran arrêté sur {}", output),
        }
    }

    pub fn status_stopped_all(&self) -> &'static str {
        match self {
            Language::Es => "Todos los fondos han sido detenidos",
            Language::En => "All wallpapers have been stopped",
            Language::Fr => "Tous les fonds d’écran ont été arrêtés",
        }
    }

    pub fn status_muted(&self) -> &'static str {
        match self {
            Language::Es => "Audio silenciado",
            Language::En => "Audio muted",
            Language::Fr => "Son coupé",
        }
    }

    pub fn status_unmuted(&self) -> &'static str {
        match self {
            Language::Es => "Audio activado",
            Language::En => "Audio enabled",
            Language::Fr => "Son activé",
        }
    }

    pub fn status_folder_removed(&self, folder: &str) -> String {
        match self {
            Language::Es => format!("Carpeta eliminada: {}", folder),
            Language::En => format!("Folder removed: {}", folder),
            Language::Fr => format!("Dossier retiré : {}", folder),
        }
    }

    pub fn status_library_refreshed(&self) -> &'static str {
        match self {
            Language::Es => "Biblioteca actualizada",
            Language::En => "Library updated",
            Language::Fr => "Bibliothèque mise à jour",
        }
    }

    pub fn status_video_added(&self, name: &str) -> String {
        match self {
            Language::Es => format!("Video añadido: {}", name),
            Language::En => format!("Video added: {}", name),
            Language::Fr => format!("Vidéo ajoutée : {}", name),
        }
    }

    pub fn status_folder_added(&self, folder: &str) -> String {
        match self {
            Language::Es => format!("Carpeta añadida: {}", folder),
            Language::En => format!("Folder added: {}", folder),
            Language::Fr => format!("Dossier ajouté : {}", folder),
        }
    }

    pub fn status_videos_dropped(&self) -> &'static str {
        match self {
            Language::Es => "Videos añadidos a la biblioteca",
            Language::En => "Videos added to library",
            Language::Fr => "Vidéos ajoutées à la bibliothèque",
        }
    }

    pub fn status_topology_updated(&self, count: usize) -> String {
        match self {
            Language::Es => format!("Topología de pantallas actualizada ({} monitores)", count),
            Language::En => format!("Display topology updated ({} displays)", count),
            Language::Fr => format!("Topologie des écrans mise à jour ({} écrans)", count),
        }
    }

    pub fn status_game_paused(&self) -> &'static str {
        match self {
            Language::Es => "Modo Juego: Fondo pausado para máximo rendimiento",
            Language::En => "Game Mode: Wallpaper paused for maximum performance",
            Language::Fr => "Mode jeu : fond d’écran mis en pause pour des performances maximales",
        }
    }

    pub fn status_game_resumed(&self) -> &'static str {
        match self {
            Language::Es => "Juego minimizado: Fondo reanudado",
            Language::En => "Game minimized: Wallpaper resumed",
            Language::Fr => "Jeu réduit : lecture du fond d’écran reprise",
        }
    }

    pub fn status_session_locked_paused(&self) -> &'static str {
        match self {
            Language::Es => "Sesión bloqueada: Fondo pausado",
            Language::En => "Session locked: Wallpaper paused",
            Language::Fr => "Session verrouillée : fond d’écran en pause",
        }
    }

    pub fn status_overview_paused(&self) -> &'static str {
        match self {
            Language::Es => "Vista general de Niri: Fondos pausados",
            Language::En => "Niri overview: Wallpapers paused",
            Language::Fr => "Vue d’ensemble de Niri : fonds d’écran en pause",
        }
    }

    pub fn status_overview_resumed(&self) -> &'static str {
        match self {
            Language::Es => "Vista general cerrada: Fondos reanudados",
            Language::En => "Overview closed: Wallpapers resumed",
            Language::Fr => "Vue d’ensemble fermée : fonds d’écran repris",
        }
    }

    pub fn dialog_pick_video(&self) -> &'static str {
        match self {
            Language::Es => "Seleccionar Video de Fondo",
            Language::En => "Select Wallpaper Video",
            Language::Fr => "Sélectionner une vidéo de fond d’écran",
        }
    }

    pub fn dialog_pick_folder(&self) -> &'static str {
        match self {
            Language::Es => "Seleccionar Carpeta con Videos",
            Language::En => "Select Folder with Videos",
            Language::Fr => "Sélectionner un dossier contenant des vidéos",
        }
    }

    #[allow(dead_code)]
    pub fn library_btn_remove_custom(&self) -> &'static str {
        match self {
            Language::Es => "Quitar",
            Language::En => "Remove",
            Language::Fr => "Retirer",
        }
    }

    #[allow(dead_code)]
    pub fn library_btn_delete_download(&self) -> &'static str {
        match self {
            Language::Es => "Eliminar",
            Language::En => "Delete",
            Language::Fr => "Supprimer",
        }
    }

    pub fn library_removed_toast(&self) -> &'static str {
        match self {
            Language::Es => "Fondo quitado de la biblioteca",
            Language::En => "Wallpaper removed from library",
            Language::Fr => "Fond d’écran supprimé de la bibliothèque",
        }
    }

    pub fn library_target_output_label(&self) -> &'static str {
        match self {
            Language::Es => "Pantalla destino:",
            Language::En => "Target Display:",
            Language::Fr => "Écran cible :",
        }
    }

    pub fn monitors_layout_title(&self) -> &'static str {
        match self {
            Language::Es => "Distribución real de monitores",
            Language::En => "Real monitor layout",
            Language::Fr => "Disposition réelle des écrans",
        }
    }

    pub fn monitors_drag_hint(&self) -> &'static str {
        match self {
            Language::Es => "Arrastra un monitor sobre otro para cambiar el orden de la lista. Las posiciones y tamaños vienen del compositor.",
            Language::En => "Drag one monitor onto another to reorder the list. Positions and sizes come from the compositor.",
            Language::Fr => "Faites glisser un écran sur un autre pour changer l’ordre de la liste. Les positions et tailles viennent du compositeur.",
        }
    }

    pub fn monitors_default_badge(&self) -> &'static str {
        match self {
            Language::Es => "Predeterminado: todos",
            Language::En => "Default: all monitors",
            Language::Fr => "Par défaut : tous les écrans",
        }
    }

    pub fn monitors_override_badge(&self) -> &'static str {
        match self {
            Language::Es => "Solo este monitor",
            Language::En => "This monitor only",
            Language::Fr => "Cet écran uniquement",
        }
    }

    pub fn monitors_apply_here(&self) -> &'static str {
        match self {
            Language::Es => "Elegir fondo para este monitor",
            Language::En => "Choose wallpaper for this monitor",
            Language::Fr => "Choisir un fond pour cet écran",
        }
    }

    pub fn monitors_apply_all(&self) -> &'static str {
        match self {
            Language::Es => "Elegir fondo para todos",
            Language::En => "Choose wallpaper for all",
            Language::Fr => "Choisir un fond pour tous",
        }
    }

    pub fn monitors_stop_all(&self) -> &'static str {
        match self {
            Language::Es => "Detener todos",
            Language::En => "Stop all",
            Language::Fr => "Tout arrêter",
        }
    }

    pub fn monitors_pause_all(&self) -> &'static str {
        match self {
            Language::Es => "Pausar todos",
            Language::En => "Pause all",
            Language::Fr => "Tout mettre en pause",
        }
    }

    pub fn monitors_identify(&self) -> &'static str {
        match self {
            Language::Es => "Identificar",
            Language::En => "Identify",
            Language::Fr => "Identifier",
        }
    }

    pub fn monitors_view_wallpaper(&self) -> &'static str {
        match self {
            Language::Es => "Ver fondo",
            Language::En => "View wallpaper",
            Language::Fr => "Voir le fond",
        }
    }

    pub fn monitors_no_outputs(&self) -> &'static str {
        match self {
            Language::Es => "No se detectaron monitores. Conecta una salida o revisa el compositor.",
            Language::En => "No monitors detected. Connect an output or check the compositor.",
            Language::Fr => "Aucun écran détecté. Connectez une sortie ou vérifiez le compositeur.",
        }
    }

    pub fn monitors_apply_to_this(&self) -> &'static str {
        match self {
            Language::Es => "Elegir pantalla",
            Language::En => "Select display",
            Language::Fr => "Sélectionner un écran",
        }
    }

    pub fn monitors_all_displays(&self) -> &'static str {
        match self {
            Language::Es => "Todas las pantallas",
            Language::En => "All Displays",
            Language::Fr => "Tous les écrans",
        }
    }

    pub fn monitors_target_active(&self) -> &'static str {
        match self {
            Language::Es => "Destino activo",
            Language::En => "Active Target",
            Language::Fr => "Cible active",
        }
    }

    pub fn scaling_fit(&self) -> &'static str {
        match self {
            Language::Es => "Ajustar",
            Language::En => "Fit",
            Language::Fr => "Ajuster",
        }
    }

    pub fn scaling_fill(&self) -> &'static str {
        match self {
            Language::Es => "Rellenar",
            Language::En => "Fill",
            Language::Fr => "Remplir",
        }
    }

    pub fn scaling_stretch(&self) -> &'static str {
        match self {
            Language::Es => "Estirar",
            Language::En => "Stretch",
            Language::Fr => "Étirer",
        }
    }

    pub fn status_scaling_changed(&self, output: &str, scaling: &str) -> String {
        match self {
            Language::Es => format!("Modo de escala para {} cambiado a '{}'", output, scaling),
            Language::En => format!("Scaling mode for {} changed to '{}'", output, scaling),
            Language::Fr => format!(
                "Mode de mise à l’échelle pour {} changé en '{}'",
                output, scaling
            ),
        }
    }

    pub fn status_output_selected(&self, output: &str) -> String {
        match self {
            Language::Es => format!("Pantalla seleccionada: {}", output),
            Language::En => format!("Selected display: {}", output),
            Language::Fr => format!("Écran sélectionné : {}", output),
        }
    }

    pub fn status_hwdec_selected(&self, hwdec: &str) -> String {
        match self {
            Language::Es => format!("Decodificador GPU: {}", hwdec),
            Language::En => format!("GPU decoder: {}", hwdec),
            Language::Fr => format!("Décodeur GPU : {}", hwdec),
        }
    }

    pub fn status_gpu_selected(&self, gpu: &str) -> String {
        let label = match gpu {
            "discrete" => match self {
                Language::Es => "Dedicada",
                Language::En => "Dedicated",
                Language::Fr => "Dédiée",
            },
            "integrated" => match self {
                Language::Es => "Integrada",
                Language::En => "Integrated",
                Language::Fr => "Intégrée",
            },
            _ => match self {
                Language::Es => "Automático",
                Language::En => "Auto",
                Language::Fr => "Auto",
            },
        };
        match self {
            Language::Es => format!("Tarjeta GPU: {}", label),
            Language::En => format!("GPU Card: {}", label),
            Language::Fr => format!("Carte graphique : {}", label),
        }
    }

    pub fn status_interval_selected(&self, interval: u64) -> String {
        match self {
            Language::Es => format!("Intervalo de rotación: {} min", interval),
            Language::En => format!("Rotation interval: {} min", interval),
            Language::Fr => format!("Intervalle de rotation : {} min", interval),
        }
    }

    pub fn status_rotation_order_selected(&self, order: &str) -> String {
        match self {
            Language::Es => format!(
                "Orden de rotación: {}",
                if order == "random" {
                    "Aleatorio"
                } else {
                    "Secuencial"
                }
            ),
            Language::En => format!(
                "Rotation order: {}",
                if order == "random" {
                    "Random"
                } else {
                    "Sequential"
                }
            ),
            Language::Fr => format!(
                "Ordre de rotation : {}",
                if order == "random" {
                    "Aléatoire"
                } else {
                    "Séquentiel"
                }
            ),
        }
    }

    pub fn status_pause_battery_enabled(&self) -> &'static str {
        match self {
            Language::Es => "Ahorro de batería activado",
            Language::En => "Battery saver enabled",
            Language::Fr => "Économie d’énergie activée",
        }
    }

    pub fn status_pause_battery_disabled(&self) -> &'static str {
        match self {
            Language::Es => "Ahorro de batería desactivado",
            Language::En => "Battery saver disabled",
            Language::Fr => "Économie d’énergie désactivée",
        }
    }

    pub fn settings_rotation_title(&self) -> &'static str {
        match self {
            Language::Es => "Rotación Automática de Fondos",
            Language::En => "Playlist Auto-Rotation",
            Language::Fr => "Rotation automatique des fonds d’écran",
        }
    }

    pub fn settings_rotation_desc(&self) -> &'static str {
        match self {
            Language::Es => "Cambia periódicamente de fondo entre los elementos de la biblioteca",
            Language::En => "Periodically switch wallpapers from your library collection",
            Language::Fr => {
                "Changer périodiquement de fond d’écran parmi ceux de votre bibliothèque"
            }
        }
    }

    pub fn settings_rotation_toggle(&self) -> &'static str {
        match self {
            Language::Es => "Activar rotación automática",
            Language::En => "Enable automatic rotation",
            Language::Fr => "Activer la rotation automatique",
        }
    }

    pub fn status_rotation_enabled(&self) -> &'static str {
        match self {
            Language::Es => "Rotación automática activada",
            Language::En => "Automatic rotation enabled",
            Language::Fr => "Rotation automatique activée",
        }
    }

    pub fn status_rotation_disabled(&self) -> &'static str {
        match self {
            Language::Es => "Rotación automática desactivada",
            Language::En => "Automatic rotation disabled",
            Language::Fr => "Rotation automatique désactivée",
        }
    }

    pub fn settings_interval_label(&self) -> &'static str {
        match self {
            Language::Es => "Intervalo de cambio:",
            Language::En => "Switch Interval:",
            Language::Fr => "Intervalle de changement :",
        }
    }

    pub fn settings_interval_custom_label(&self) -> &'static str {
        match self {
            Language::Es => "Personalizado:",
            Language::En => "Custom:",
            Language::Fr => "Personnalisé :",
        }
    }

    pub fn settings_interval_custom_placeholder(&self) -> &'static str {
        match self {
            Language::Es => "ej. 45",
            Language::En => "e.g. 45",
            Language::Fr => "ex. : 45",
        }
    }

    pub fn settings_interval_mins_unit(&self) -> &'static str {
        match self {
            Language::Es => "min",
            Language::En => "min",
            Language::Fr => "min",
        }
    }

    pub fn settings_interval_apply(&self) -> &'static str {
        match self {
            Language::Es => "Aplicar",
            Language::En => "Apply",
            Language::Fr => "Appliquer",
        }
    }

    pub fn status_interval_invalid(&self) -> &'static str {
        match self {
            Language::Es => "El intervalo debe ser un número entre 1 y 1440 minutos",
            Language::En => "Interval must be a number between 1 and 1440 minutes",
            Language::Fr => "L’intervalle doit être un nombre compris entre 1 et 1 440 minutes",
        }
    }

    pub fn settings_order_label(&self) -> &'static str {
        match self {
            Language::Es => "Orden de reproducción:",
            Language::En => "Playback Order:",
            Language::Fr => "Ordre de lecture :",
        }
    }

    pub fn settings_order_random(&self) -> &'static str {
        match self {
            Language::Es => "Aleatorio",
            Language::En => "Random",
            Language::Fr => "Aléatoire",
        }
    }

    pub fn settings_order_seq(&self) -> &'static str {
        match self {
            Language::Es => "Secuencial",
            Language::En => "Sequential",
            Language::Fr => "Séquentiel",
        }
    }

    pub fn settings_rotation_source_label(&self) -> &'static str {
        match self {
            Language::Es => "Fondos a rotar:",
            Language::En => "Wallpapers to rotate:",
            Language::Fr => "Fonds d’écran à faire tourner :",
        }
    }

    pub fn settings_rotation_source_all(&self) -> &'static str {
        match self {
            Language::Es => "Toda la biblioteca",
            Language::En => "All wallpapers",
            Language::Fr => "Tous les fonds d’écran",
        }
    }

    pub fn settings_rotation_source_favs(&self) -> &'static str {
        match self {
            Language::Es => "Solo favoritos",
            Language::En => "Only favorites",
            Language::Fr => "Favoris uniquement",
        }
    }

    pub fn settings_rotation_favs_empty_hint(&self) -> &'static str {
        match self {
            Language::Es => "⚠️ No tienes favoritos guardados. Se rotará toda la biblioteca hasta que marques fondos con el corazón.",
            Language::En => "⚠️ No favorites saved yet. All wallpapers will cycle until you mark favorites with the heart.",
            Language::Fr => "⚠️ Aucun favori enregistré pour le moment. Tous les fonds d’écran défileront jusqu’à ce que vous en marquiez comme favoris avec le cœur.",
        }
    }

    pub fn settings_rotation_favs_active_hint(&self, count: usize) -> String {
        match self {
            Language::Es => format!("✓ Rotando únicamente entre tus {} fondos favoritos.", count),
            Language::En => format!("✓ Cycling only between your {} favorite wallpapers.", count),
            Language::Fr => format!(
                "✓ Rotation uniquement entre vos {} fonds d’écran favoris.",
                count
            ),
        }
    }

    pub fn status_rotation_source_all(&self) -> &'static str {
        match self {
            Language::Es => "Rotación configurada: toda la biblioteca",
            Language::En => "Rotation set: all wallpapers",
            Language::Fr => "Rotation configurée : tous les fonds d’écran",
        }
    }

    pub fn status_rotation_source_favs(&self) -> &'static str {
        match self {
            Language::Es => "Rotación configurada: solo favoritos",
            Language::En => "Rotation set: only favorites",
            Language::Fr => "Rotation configurée : favoris uniquement",
        }
    }

    pub fn status_favs_rotation_started(&self, mins: u64) -> String {
        match self {
            Language::Es => format!("Rotación de favoritos activada (cada {} min)", mins),
            Language::En => format!("Favorites rotation enabled (every {} min)", mins),
            Language::Fr => format!("Rotation des favoris activée (toutes les {} min)", mins),
        }
    }

    pub fn library_favs_rotation_title(&self) -> &'static str {
        match self {
            Language::Es => "Rotación de Favoritos",
            Language::En => "Favorites Rotation",
            Language::Fr => "Rotation des favoris",
        }
    }

    pub fn library_favs_banner_desc_idle(&self, count: usize) -> String {
        match self {
            Language::Es => format!(
                "Reproduce automáticamente solo tus {} fondos favoritos en bucle",
                count
            ),
            Language::En => format!(
                "Automatically cycle only through your {} favorite wallpapers",
                count
            ),
            Language::Fr => format!(
                "Faire défiler automatiquement uniquement vos {} fonds d’écran favoris",
                count
            ),
        }
    }

    pub fn library_favs_banner_desc_active(
        &self,
        count: usize,
        interval: u64,
        is_random: bool,
    ) -> String {
        let order_str = if is_random {
            match self {
                Language::Es => "Aleatorio",
                Language::En => "Random",
                Language::Fr => "Aléatoire",
            }
        } else {
            match self {
                Language::Es => "Secuencial",
                Language::En => "Sequential",
                Language::Fr => "Séquentiel",
            }
        };
        match self {
            Language::Es => format!(
                "Rotando {} favoritos: cada {} min • {}",
                count, interval, order_str
            ),
            Language::En => format!(
                "Rotating {} favorites: every {} min • {}",
                count, interval, order_str
            ),
            Language::Fr => format!(
                "Rotation de {} favoris : toutes les {} min • {}",
                count, interval, order_str
            ),
        }
    }

    pub fn library_favs_start_rotation(&self) -> &'static str {
        match self {
            Language::Es => "Rotar favoritos",
            Language::En => "Rotate favorites",
            Language::Fr => "Faire tourner les favoris",
        }
    }

    pub fn library_favs_stop_rotation(&self) -> &'static str {
        match self {
            Language::Es => "Detener rotación",
            Language::En => "Stop rotation",
            Language::Fr => "Arrêter la rotation",
        }
    }

    pub fn library_favs_rotation_settings(&self) -> &'static str {
        match self {
            Language::Es => "Ajustes de rotación",
            Language::En => "Rotation settings",
            Language::Fr => "Paramètres de rotation",
        }
    }

    pub fn settings_scaling_label(&self) -> &'static str {
        match self {
            Language::Es => "Ajuste de imagen",
            Language::En => "Image scaling",
            Language::Fr => "Mise à l’échelle",
        }
    }

    pub fn settings_hwdec_title(&self) -> &'static str {
        match self {
            Language::Es => "Aceleración de Hardware (GPU)",
            Language::En => "Hardware Acceleration (GPU)",
            Language::Fr => "Accélération matérielle (GPU)",
        }
    }

    pub fn settings_hwdec_desc(&self) -> &'static str {
        match self {
            Language::Es => "Configura el decodificador de video para mpv (VA-API / NVDEC)",
            Language::En => "Configure the video hardware decoder for mpv (VA-API / NVDEC)",
            Language::Fr => "Configurer le décodeur matériel vidéo de mpv (VA-API / NVDEC)",
        }
    }

    pub fn settings_gpu_card_title(&self) -> &'static str {
        match self {
            Language::Es => "Adaptador Gráfico (GPU)",
            Language::En => "Graphics Adapter (GPU)",
            Language::Fr => "Adaptateur graphique (GPU)",
        }
    }

    pub fn settings_gpu_card_desc(&self) -> &'static str {
        match self {
            Language::Es => "Selecciona qué tarjeta gráfica ejecuta el renderizado en portátiles con gráficos híbridos (Intel / AMD / NVIDIA)",
            Language::En => "Select which graphics card renders video on notebooks with hybrid graphics (Intel / AMD / NVIDIA)",
            Language::Fr => "Sélectionnez la carte graphique utilisée pour le rendu vidéo sur les ordinateurs portables à graphiques hybrides (Intel / AMD / NVIDIA)",
        }
    }

    pub fn settings_gpu_auto(&self) -> &'static str {
        match self {
            Language::Es => "Automático",
            Language::En => "Auto",
            Language::Fr => "Auto",
        }
    }

    pub fn settings_gpu_integrated(&self) -> &'static str {
        match self {
            Language::Es => "Integrada (Ahorro)",
            Language::En => "Integrated (Power Saver)",
            Language::Fr => "Intégrée (économie d’énergie)",
        }
    }

    pub fn settings_gpu_discrete(&self) -> &'static str {
        match self {
            Language::Es => "Dedicada (Rendimiento)",
            Language::En => "Dedicated (Performance)",
            Language::Fr => "Dédiée (performances)",
        }
    }

    pub fn settings_battery_title(&self) -> &'static str {
        match self {
            Language::Es => "Ahorro de Batería en Portátiles",
            Language::En => "Laptop Battery Saver",
            Language::Fr => "Économie d’énergie sur batterie",
        }
    }

    pub fn settings_battery_desc(&self) -> &'static str {
        match self {
            Language::Es => "Suspende automáticamente la reproducción al usar la batería del portátil",
            Language::En => "Automatically suspend playback when running on laptop battery",
            Language::Fr => "Suspendre automatiquement la lecture lorsque l’ordinateur portable fonctionne sur batterie",
        }
    }

    pub fn status_battery_paused(&self) -> &'static str {
        match self {
            Language::Es => "Ahorro de batería: Fondo pausado (0% GPU)",
            Language::En => "Battery saver: Wallpaper paused (0% GPU)",
            Language::Fr => "Économie d’énergie : fond d’écran en pause (0 % GPU)",
        }
    }

    pub fn status_battery_resumed(&self) -> &'static str {
        match self {
            Language::Es => "Alimentación por CA conectada: Fondo reanudado",
            Language::En => "AC power connected: Wallpaper resumed",
            Language::Fr => "Alimentation secteur connectée : lecture du fond d’écran reprise",
        }
    }

    #[allow(dead_code)]
    pub fn bar_volume(&self) -> &'static str {
        match self {
            Language::Es => "Volumen",
            Language::En => "Volume",
            Language::Fr => "Volume",
        }
    }

    // --- Quick Switcher HUD Settings ---
    pub fn settings_switcher_title(&self) -> &'static str {
        match self {
            Language::Es => "Selector Rápido HUD",
            Language::En => "Quick Switcher HUD",
            Language::Fr => "HUD de sélection rapide",
        }
    }

    pub fn settings_switcher_desc(&self) -> &'static str {
        match self {
            Language::Es => "Ventana emergente minimalista centrada para cambiar fondos al instante",
            Language::En => "Minimalist centered popup window to switch wallpapers instantly",
            Language::Fr => "Fenêtre contextuelle minimaliste centrée pour changer instantanément de fond d’écran",
        }
    }

    pub fn settings_switcher_tray_click_label(&self) -> &'static str {
        match self {
            Language::Es => "Clic en el icono de la barra:",
            Language::En => "Panel icon click action:",
            Language::Fr => "Action au clic sur l’icône du panneau :",
        }
    }

    pub fn settings_switcher_tray_opt_switcher(&self) -> &'static str {
        match self {
            Language::Es => "Selector Rápido HUD",
            Language::En => "Quick Switcher HUD",
            Language::Fr => "HUD de sélection rapide",
        }
    }

    pub fn settings_switcher_tray_opt_main(&self) -> &'static str {
        match self {
            Language::Es => "Ventana Principal",
            Language::En => "Main Window",
            Language::Fr => "Fenêtre principale",
        }
    }

    pub fn settings_switcher_filter_label(&self) -> &'static str {
        match self {
            Language::Es => "Fondos en el carrusel:",
            Language::En => "Wallpapers in carousel:",
            Language::Fr => "Fonds d’écran dans le carrousel :",
        }
    }

    pub fn settings_switcher_filter_all(&self) -> &'static str {
        match self {
            Language::Es => "Toda la biblioteca",
            Language::En => "All wallpapers",
            Language::Fr => "Tous les fonds d’écran",
        }
    }

    pub fn settings_switcher_filter_favs(&self) -> &'static str {
        match self {
            Language::Es => "Solo favoritos",
            Language::En => "Only favorites",
            Language::Fr => "Favoris uniquement",
        }
    }

    pub fn settings_switcher_shortcut_title(&self) -> &'static str {
        match self {
            Language::Es => "Atajo global en GNOME o COSMIC",
            Language::En => "Global shortcut in GNOME or COSMIC",
            Language::Fr => "Raccourci global dans GNOME ou COSMIC",
        }
    }

    pub fn settings_switcher_shortcut_desc(&self) -> &'static str {
        match self {
            Language::Es => "Abre el HUD con una combinación de teclas: añade el comando en Atajos personalizados del teclado en los ajustes de GNOME o COSMIC:",
            Language::En => "Open the HUD with a key combo by adding this command under Custom Keyboard Shortcuts in GNOME or COSMIC Settings:",
            Language::Fr => "Ouvrez le HUD avec un raccourci clavier en ajoutant cette commande aux raccourcis clavier personnalisés de GNOME ou COSMIC :",
        }
    }

    pub fn settings_section_integration(&self) -> &'static str {
        match self {
            Language::Es => "Integración Niri / DMS",
            Language::En => "Niri / DMS integration",
            Language::Fr => "Intégration Niri / DMS",
        }
    }

    pub fn settings_integration_desc(&self) -> &'static str {
        match self {
            Language::Es => "Aura detecta Niri y DMS sin depender de ellos. Niri conserva el control de la posición de la ventana.",
            Language::En => "Aura detects Niri and DMS without depending on them. Niri keeps ownership of window placement.",
            Language::Fr => "Aura détecte Niri et DMS sans en dépendre. Niri conserve le placement des fenêtres.",
        }
    }

    pub fn settings_integration_niri_ok(&self) -> &'static str {
        match self {
            Language::Es => "Niri detectado: layout, escala y rotación reales",
            Language::En => "Niri detected: real layout, scale and rotation",
            Language::Fr => "Niri détecté : disposition, échelle et rotation réelles",
        }
    }

    pub fn settings_integration_niri_missing(&self) -> &'static str {
        match self {
            Language::Es => "Niri no detectado (se usa la detección genérica de salidas)",
            Language::En => "Niri not detected (generic output detection is used)",
            Language::Fr => "Niri non détecté (détection générique des sorties)",
        }
    }

    pub fn settings_integration_dms_ok(&self) -> &'static str {
        match self {
            Language::Es => "DMS / Quickshell detectado",
            Language::En => "DMS / Quickshell detected",
            Language::Fr => "DMS / Quickshell détecté",
        }
    }

    pub fn settings_integration_dms_missing(&self) -> &'static str {
        match self {
            Language::Es => "DMS no detectado",
            Language::En => "DMS not detected",
            Language::Fr => "DMS non détecté",
        }
    }

    pub fn settings_integration_underlay(&self) -> &'static str {
        match self {
            Language::Es => "Sincronizar póster en la capa DMS (best-effort)",
            Language::En => "Sync poster in the DMS layer (best-effort)",
            Language::Fr => "Synchroniser l’affiche dans la couche DMS (best-effort)",
        }
    }

    pub fn settings_integration_underlay_desc(&self) -> &'static str {
        match self {
            Language::Es => "Opcional. La versión actual de DMS no expone un IPC estable para fijar una ruta; si no está disponible, Aura mantiene su transición propia.",
            Language::En => "Optional. Current DMS does not expose a stable IPC to set a path; when unavailable, Aura keeps its own transition.",
            Language::Fr => "Facultatif. La version actuelle de DMS n’expose pas d’IPC stable pour définir un chemin ; sinon Aura utilise sa propre transition.",
        }
    }

    pub fn settings_integration_copy_rule(&self) -> &'static str {
        match self {
            Language::Es => "Copiar layer-rule",
            Language::En => "Copy layer-rule",
            Language::Fr => "Copier la layer-rule",
        }
    }

    pub fn settings_integration_copy_window_rule(&self) -> &'static str {
        match self {
            Language::Es => "Copiar window-rule",
            Language::En => "Copy window-rule",
            Language::Fr => "Copier la window-rule",
        }
    }

    pub fn settings_integration_copy_shortcuts(&self) -> &'static str {
        match self {
            Language::Es => "Copiar atajos",
            Language::En => "Copy shortcuts",
            Language::Fr => "Copier les raccourcis",
        }
    }

    pub fn settings_integration_copy_autostart(&self) -> &'static str {
        match self {
            Language::Es => "Copiar autostart",
            Language::En => "Copy autostart",
            Language::Fr => "Copier l’autostart",
        }
    }

    pub fn settings_switcher_copy_cmd(&self) -> &'static str {
        match self {
            Language::Es => "Copiar comando",
            Language::En => "Copy command",
            Language::Fr => "Copier la commande",
        }
    }

    pub fn settings_switcher_copied(&self) -> &'static str {
        match self {
            Language::Es => "Comando copiado al portapapeles: aura switcher",
            Language::En => "Command copied to clipboard: aura switcher",
            Language::Fr => "Commande copiée dans le presse-papiers : aura switcher",
        }
    }

    pub fn settings_switcher_position_label(&self) -> &'static str {
        match self {
            Language::Es => "Posición en pantalla:",
            Language::En => "Screen position:",
            Language::Fr => "Position à l’écran :",
        }
    }

    pub fn settings_switcher_pos_top(&self) -> &'static str {
        match self {
            Language::Es => "Arriba",
            Language::En => "Top",
            Language::Fr => "Haut",
        }
    }

    pub fn settings_switcher_pos_bottom(&self) -> &'static str {
        match self {
            Language::Es => "Abajo",
            Language::En => "Bottom",
            Language::Fr => "Bas",
        }
    }

    pub fn settings_switcher_pos_left(&self) -> &'static str {
        match self {
            Language::Es => "Izquierda",
            Language::En => "Left",
            Language::Fr => "Gauche",
        }
    }

    pub fn settings_switcher_pos_right(&self) -> &'static str {
        match self {
            Language::Es => "Derecha",
            Language::En => "Right",
            Language::Fr => "Droite",
        }
    }

    pub fn settings_switcher_style_label(&self) -> &'static str {
        match self {
            Language::Es => "Estilo del HUD:",
            Language::En => "HUD Style:",
            Language::Fr => "Style du HUD :",
        }
    }

    pub fn settings_switcher_style_classic(&self) -> &'static str {
        match self {
            Language::Es => "Clásico",
            Language::En => "Classic",
            Language::Fr => "Classique",
        }
    }

    pub fn settings_switcher_style_cinematic(&self) -> &'static str {
        match self {
            Language::Es => "Cinemático",
            Language::En => "Cinematic",
            Language::Fr => "Cinématique",
        }
    }

    pub fn settings_switcher_style_cinematic_desc(&self) -> &'static str {
        match self {
            Language::Es => {
                "Carrusel inmersivo a pantalla completa con tarjetas inclinadas y efecto coverflow"
            }
            Language::En => "Fullscreen immersive carousel with skewed cards and coverflow effect",
            Language::Fr => {
                "Carrousel immersif en plein écran avec cartes inclinées et effet coverflow"
            }
        }
    }

    pub fn settings_switcher_style_honeycomb(&self) -> &'static str {
        match self {
            Language::Es => "Panal de Abeja",
            Language::En => "Honeycomb",
            Language::Fr => "Nid d’abeilles",
        }
    }

    pub fn settings_switcher_style_honeycomb_desc(&self) -> &'static str {
        match self {
            Language::Es => {
                "Malla hexagonal panorámica con miniaturas optimizadas y desplazamiento fluido"
            }
            Language::En => {
                "Panoramic hexagonal mesh with optimized thumbnails and fluid scrolling"
            }
            Language::Fr => {
                "Grille hexagonale panoramique avec miniatures optimisées et défilement fluide"
            }
        }
    }

    #[allow(dead_code)]
    pub fn switcher_cinematic_nav_hint(&self) -> &'static str {
        match self {
            Language::Es => {
                "← → Navegar  ·  Enter Aplicar  ·  Espacio Pausa  ·  F Favorito  ·  Esc Salir"
            }
            Language::En => {
                "← → Navigate  ·  Enter Apply  ·  Space Pause  ·  F Favorite  ·  Esc Exit"
            }
            Language::Fr => {
                "← → Naviguer  ·  Entrée Appliquer  ·  Espace Pause  ·  F Favori  ·  Échap Quitter"
            }
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
        assert_eq!(Language::from_code("fr"), Language::Fr);
        assert_eq!(Language::from_code("FR"), Language::Fr);
        assert_eq!(Language::from_code("french"), Language::Fr);
        assert_eq!(Language::from_code("français"), Language::Fr);
        assert_eq!(Language::from_code(""), Language::En);
    }

    #[test]
    fn test_language_code() {
        assert_eq!(Language::Es.code(), "es");
        assert_eq!(Language::En.code(), "en");
        assert_eq!(Language::Fr.code(), "fr");
    }

    #[test]
    fn test_translations_non_empty() {
        for lang in &[Language::Es, Language::En, Language::Fr] {
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
            assert!(!lang.tray_switcher().is_empty());
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
            assert!(!lang.explore_source_minimalistic().is_empty());
            assert!(!lang.explore_source_motionbgs().is_empty());
            assert!(!lang.explore_btn_cancel().is_empty());
            assert!(!lang.explore_motionbgs_subtitle().is_empty());
            assert!(!lang.explore_search_motionbgs_placeholder().is_empty());
            assert!(!lang.explore_cat_games().is_empty());
            assert!(!lang.explore_cat_nature().is_empty());
            assert!(!lang.explore_cat_space().is_empty());
            assert!(!lang.explore_cat_cars().is_empty());
            assert!(!lang.explore_cat_superhero().is_empty());
            assert!(!lang.explore_cat_fantasy().is_empty());
            assert!(!lang.explore_cat_technology().is_empty());
            assert!(!lang.explore_minimalistic_subtitle().is_empty());
            assert!(!lang.explore_search_minimal_placeholder().is_empty());
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
            assert!(!lang.monitors_target_active().is_empty());
            assert!(!lang.scaling_fit().is_empty());
            assert!(!lang.scaling_fill().is_empty());
            assert!(!lang.scaling_stretch().is_empty());
            assert!(!lang.status_pause_battery_enabled().is_empty());
            assert!(!lang.status_pause_battery_disabled().is_empty());
            assert!(!lang.settings_rotation_title().is_empty());
            assert!(!lang.settings_rotation_desc().is_empty());
            assert!(!lang.settings_rotation_toggle().is_empty());
            assert!(!lang.settings_interval_label().is_empty());
            assert!(!lang.settings_interval_custom_label().is_empty());
            assert!(!lang.settings_interval_custom_placeholder().is_empty());
            assert!(!lang.settings_interval_mins_unit().is_empty());
            assert!(!lang.settings_interval_apply().is_empty());
            assert!(!lang.status_interval_invalid().is_empty());
            assert!(!lang.settings_order_label().is_empty());
            assert!(!lang.settings_order_random().is_empty());
            assert!(!lang.settings_order_seq().is_empty());
            assert!(!lang.settings_rotation_source_label().is_empty());
            assert!(!lang.settings_rotation_source_all().is_empty());
            assert!(!lang.settings_rotation_source_favs().is_empty());
            assert!(!lang.settings_rotation_favs_empty_hint().is_empty());
            assert!(!lang.status_rotation_source_all().is_empty());
            assert!(!lang.status_rotation_source_favs().is_empty());
            assert!(!lang.library_favs_rotation_title().is_empty());
            assert!(!lang.library_favs_banner_desc_idle(1).is_empty());
            assert!(!lang.library_favs_start_rotation().is_empty());
            assert!(!lang.library_favs_stop_rotation().is_empty());
            assert!(!lang.library_favs_rotation_settings().is_empty());
            assert!(!lang.settings_hwdec_title().is_empty());
            assert!(!lang.settings_hwdec_desc().is_empty());
            assert!(!lang.settings_gpu_card_title().is_empty());
            assert!(!lang.settings_gpu_card_desc().is_empty());
            assert!(!lang.settings_gpu_auto().is_empty());
            assert!(!lang.settings_gpu_integrated().is_empty());
            assert!(!lang.settings_gpu_discrete().is_empty());
            assert!(!lang.status_gpu_selected("discrete").is_empty());
            assert!(!lang.settings_battery_title().is_empty());
            assert!(!lang.settings_battery_desc().is_empty());
            assert!(!lang.status_battery_paused().is_empty());
            assert!(!lang.status_battery_resumed().is_empty());
            assert!(!lang.settings_auto_pause().is_empty());
            assert!(!lang.settings_auto_pause_desc().is_empty());
            assert!(!lang.bar_volume().is_empty());
            assert!(!lang.bar_prev().is_empty());
            assert!(!lang.bar_next().is_empty());
            assert!(!lang.settings_switcher_title().is_empty());
            assert!(!lang.settings_switcher_desc().is_empty());
            assert!(!lang.settings_switcher_tray_click_label().is_empty());
            assert!(!lang.settings_switcher_tray_opt_switcher().is_empty());
            assert!(!lang.settings_switcher_tray_opt_main().is_empty());
            assert!(!lang.settings_switcher_filter_label().is_empty());
            assert!(!lang.settings_switcher_filter_all().is_empty());
            assert!(!lang.settings_switcher_filter_favs().is_empty());
            assert!(!lang.settings_switcher_shortcut_title().is_empty());
            assert!(!lang.settings_switcher_shortcut_desc().is_empty());
            assert!(!lang.settings_switcher_copy_cmd().is_empty());
            assert!(!lang.settings_switcher_copied().is_empty());
            assert!(!lang.settings_switcher_position_label().is_empty());
            assert!(!lang.settings_switcher_pos_top().is_empty());
            assert!(!lang.settings_switcher_pos_bottom().is_empty());
            assert!(!lang.settings_switcher_pos_left().is_empty());
            assert!(!lang.settings_switcher_pos_right().is_empty());
            assert!(!lang.settings_switcher_style_label().is_empty());
            assert!(!lang.settings_switcher_style_classic().is_empty());
            assert!(!lang.settings_switcher_style_cinematic().is_empty());
            assert!(!lang.settings_switcher_style_cinematic_desc().is_empty());
            assert!(!lang.settings_switcher_style_honeycomb().is_empty());
            assert!(!lang.settings_switcher_style_honeycomb_desc().is_empty());
            assert!(!lang.switcher_cinematic_nav_hint().is_empty());
        }
    }

    #[test]
    fn test_dynamic_formatting() {
        assert_eq!(Language::Es.header_wallpapers_count(5), "(5 fondos)");
        assert_eq!(Language::En.header_wallpapers_count(5), "(5 wallpapers)");
        assert_eq!(Language::Fr.header_wallpapers_count(5), "(5 fonds d’écran)");
        assert!(Language::Es
            .status_applied("DP-1", "ocean.mp4")
            .contains("DP-1"));
        assert!(Language::En
            .status_applied("DP-1", "ocean.mp4")
            .contains("ocean.mp4"));
        assert!(Language::Es.about_version_info().contains("COSMIC Desktop"));
        assert!(Language::En.about_version_info().contains("COSMIC Desktop"));
        assert!(Language::Es.settings_autostart().contains("COSMIC Desktop"));
        assert!(Language::En.settings_autostart().contains("COSMIC Desktop"));
    }

    #[test]
    fn test_clean_language_titles() {
        assert_eq!(Language::Es.settings_lang_title(), "Idioma");
        assert_eq!(Language::En.settings_lang_title(), "Language");
        assert_eq!(Language::Fr.settings_lang_title(), "Langue");
        assert!(!Language::Es.settings_lang_title().contains('/'));
        assert!(!Language::En.settings_lang_title().contains('/'));
        assert!(!Language::Fr.settings_lang_title().contains('/'));
        assert_eq!(Language::Es.library_btn_load_more(), "Cargar más fondos");
        assert_eq!(Language::En.library_btn_load_more(), "Load more wallpapers");
        assert_eq!(
            Language::Fr.library_btn_load_more(),
            "Charger plus de fonds d’écran"
        );
    }
}
