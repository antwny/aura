# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.3] - 2026-09-10

### Changed
- **About View UI Refinements & Visual Polish**:
  - Simplified GitHub and YouTube action buttons with clean, concise labels ("Github", "YouTube") and removed extraneous icons.
  - Kept PayPal donation button intact with iconic heart glyph.
  - Removed lengthy license description paragraph beneath GPL-3.0 for a cleaner and more readable details card.
  - Removed icon box next to the "Actualizaciones de Software" heading to ensure aesthetic harmony with the rest of the application sections.
  - Streamlined version status typography (`v1.1.3 • Tu versión está al día`), removing the emoji checkmark.

## [1.1.2] - 2026-09-10

### Added
- **Integrated In-App GUI Updater**:
  - Automatic background update check on application startup querying GitHub Releases API with zero impact on cold startup times.
  - Non-intrusive floating action toast notification (`🚀 ¡Nueva versión disponible! [ Actualizar ]`) using `cosmic::widget::toaster`.
  - Dedicated Software Updates card in the "Acerca de" (About) view showing current status, release notes summary preview, manual "Buscar actualizaciones" button, and one-click atomic update & restart (`[ Actualizar a v1.1.2 ]` -> `[ Reiniciar Aura ]`).
  - Automatic Flatpak runtime detection directing users to the COSMIC Store / Flathub when running sandboxed.
- **White Monochrome Symbolic Action Icons**:
  - Added clean monochrome symbolic icons to external links in the About view (`web-browser-symbolic` for GitHub, `video-x-generic-symbolic` for YouTube, and `emblem-favorite-symbolic` for Donate).
  - Standardized styling and removed emoji clash to harmonize with Pop!_OS COSMIC design guidelines.

## [1.1.1] - 2026-09-10

### Fixed
- **Human-Readable Bing Wallpaper Naming**:
  - Bing wallpapers now retain their descriptive names when downloaded (e.g. `Fields of gold.jpg` instead of `bing_adfdb411e...jpg`), displaying clean titles in Library, File Manager, and Now Playing bar.
  - Implemented cross-platform title sanitization (`clean_title_filename`) for safe storage across Linux, NTFS, and FAT filesystems.
- **Automatic Legacy Wallpaper Migration**:
  - Automatically identifies legacy `bing_<hash>.jpg` files in the user's wallpaper directory and renames them to their human titles using cached metadata archives.
  - Cleans up obsolete duplicate files from legacy hidden data paths.
- **Instant Thumbnail Linking**:
  - Pre-links cached online thumbnails upon download to avoid frame extraction delays when first viewing downloaded wallpapers in Library.

## [1.1.0] - 2026-09-10

### Added
- **Multi-Monitor Topology & Output Selector**:
  - Per-display target output selector chips in Library view (`*` all, or specific monitor).
  - Direct "Elegir pantalla" (Set as target) button in Monitors canvas.
  - Per-output child process isolation in `mpvpaper` engine without global `pkill`.
- **Multimedia Audio Controls**:
  - Global bottom Now Playing bar volume slider (0-100%) and percentage readout.
  - Instant audio mute toggle with previous volume level restoration memory.
  - Engine level `--volume={vol}` dynamic argument generation.
- **Floating Toaster Notifications (Overlay)**:
  - Integration with official `cosmic::widget::toaster` floating overlay.
  - 100% elimination of top banner vertical layout shift (Cumulative Layout Shift = 0).
  - Interactive toast action buttons (e.g. `[ Ver en archivos ]` upon downloading wallpapers).
- **User-Accessible Wallpapers Directory**:
  - Online downloads now save directly to standard user pictures folder (`~/Pictures/Wallpapers/Aura` / `~/Imágenes/Wallpapers/Aura`).
  - Automatic migration of any legacy wallpapers from hidden `~/.local/share/aura/` or Flatpak data paths.
  - "Abrir carpeta en el Gestor de Archivos" header button in Library and quick folder shortcut on each wallpaper card.
- **Advanced Engine & Power Settings**:
  - Automatic playlist wallpaper rotation with configurable timer (minutes) and order (Random or Sequential).
  - Hardware-accelerated GPU decoding selector (`auto-safe`, `vaapi`, `nvdec`).
  - Laptop battery saver automatically pausing wallpapers when unplugged (via UPower D-Bus).
  - Real-time non-blocking Feral GameMode querying to suspend wallpaper rendering while gaming.
- **CLI & Auto-Update Subcommands**:
  - `aura check-update`: Queries GitHub Releases API to verify if a newer version is available.
  - `aura update`: One-command atomic binary update and desktop database refresh directly from GitHub Releases.
- **Enhanced Flatpak Manifest**:
  - Integrated PulseAudio/PipeWire audio socket (`--socket=pulseaudio`).
  - System bus permissions for UPower (`--system-talk-name=org.freedesktop.UPower`) and GameMode (`com.feralinteractive.GameMode`).
  - Direct COSMIC theme configuration and autostart filesystem access.

### Changed
- Refactored monolithic `src/app.rs` into modular UI architecture (`src/ui/library.rs`, `src/ui/monitors.rs`, `src/ui/settings.rs`, `src/ui/bar.rs`, `src/ui/explore.rs`, `src/ui/about.rs`).
- Replaced dangerous global `pkill -x mpvpaper` with direct POSIX signal management (`libc::kill` with `SIGSTOP`, `SIGCONT`, and `SIGTERM`).
- Differentiated online download deletion from user-imported video unlinking to guarantee zero accidental user data loss.

## [1.0.0] - 2026-09-05

### Added
- **Official Flathub & COSMIC Store Release**:
  - Official Flatpak packaging manifest using System76 `com.system76.Cosmic.BaseApp` runtime and Wayland layer-shell protocol.
  - AppStream metainfo specification (`io.github.antwny.aura.metainfo.xml`) with `com.system76.CosmicApplication` categorization.
  - Standardized showcase screenshots adhering to Flathub store guidelines (16:9 aspect, window decorations, no maximization).
  - Offline Cargo dependency manifest (`cargo-sources.json`) and architecture configuration (`flathub.json`) targeting `x86_64`.
  - First production GA release consolidating pure Rust live wallpaper management, online 4K catalogs, and COSMIC desktop auto-theming.

## [0.2.0] - 2026-09-04

### Added
- **Online Wallpaper Explorer**:
  - Integrated Microsoft Bing Daily UHD archive with infinite pagination.
  - Integrated Wallhaven 4K/UHD API with categories (All, Anime, Nature/General) and sorting (Top Rated, Hot, Random).
  - Multi-resolution filtering for Wallhaven (All, 4K UHD, 2K QHD, Ultrawide 21:9).
  - Asynchronous non-blocking download with local caching and morphing action buttons.
- **Unified Smart Library**:
  - Quick filter bar with real-time counters: `[ Todos ]`, `[ Animados ]`, `[ Estáticos ]`, `[ Descargados ]`.
  - In-app wallpaper deletion and unlinking (`user-trash-symbolic`) with auto-stop if currently playing.
  - Automatic self-healing scanner for downloaded wallpapers.
- **Static Wallpaper Support**:
  - Full playback and multi-monitor canvas preview for static images (`.jpg`, `.png`, `.webp`).
  - True 0.0% CPU usage optimization for static images using `--image-display-duration=inf` and `--pause=yes`.
- **System Integration & Engine**:
  - Native Wayland auto-pause (`-p` / `--auto-pause`) pausing mpvpaper when windows obscure the wallpaper.
  - CLI flags: `aura --version`, `aura -v`, `aura version`.
  - PayPal donation support in About page and README.
  - Standardized pure COSMIC symbolic vector icons across all UI views (zero emojis).

## [0.1.0] - 2026-09-04

### Added
- Initial release of Aura live wallpaper manager for Pop!_OS COSMIC Desktop.
- 100% native Rust engine using `libcosmic`, `wgpu`, and `mpvpaper`.
- Hardware-accelerated GPU decoding with auto-detection (`auto-safe`, `vaapi`, `nvdec`).
- Dynamic COSMIC auto-theming (palette extraction syncing with system accent).
- Multi-monitor interactive layout and scaling control (Fit, Fill, Stretch).
- Asynchronous thumbnail generation engine with SHA-256 caching.
- System tray integration via D-Bus StatusNotifierItem (ksni).
- Smart Pause mode detecting fullscreen games and heavy processes.
- Command-line interface (`aura next`, `aura prev`, `aura toggle-pause`, `aura stop`, `aura apply`, `aura status`).
- Bilingual localization (Spanish & English) with auto-detection.
