# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.3] - 2026-09-21

### Added
- **Wallpaper Favorites System**:
  - Added favorite heart toggle button on library cards with COSMIC accent color highlighting (`Button::Suggested`) when active.
  - Dedicated "Favoritos" category filter in the Library toolbar with real-time count.
  - Silent, instant state toggling with persistent configuration in `config.json`.
- **Library Sorting & Responsive Navigation**:
  - Added sorting toggle by "Más recientes" (newest first) and "Más antiguos" (oldest first).
  - Horizontal smooth scrolling for category chips and monitor selectors across Library and Explore views.
  - Centered responsive card grid alignment across ultra-wide, standard, and tiled window dimensions.

### Fixed
- **COSMIC Desktop Icons & Right-Click Accessibility (PR #3)**:
  - Switched `mpvpaper` layer-shell target from `bottom` to `background`.
  - Ensures COSMIC desktop icons (`cosmic-files-applet`) and desktop context menus remain fully visible and clickable above video wallpapers (thanks to @gavindsouza).
- **Persistent Background Wallpaper Process (Fedora, Pop!_OS, CachyOS, Arch Linux)**:
  - Resolved an issue where closing the Aura application window or terminal tab terminated active video wallpapers on Fedora and other distributions.
  - Decoupled `mpvpaper` execution into an independent user-level systemd scope (`systemd-run --user --scope --quiet`) with automatic fallback to direct execution on non-systemd systems, isolating the wallpaper engine from transient desktop cgroups and `KillMode=control-group`.
  - Added POSIX session detachment (`pre_exec` with `setsid()` and `SIGHUP` signal ignoring), ensuring that closing the terminal or shell tab from which Aura was launched never interrupts active wallpapers.
  - Removed explicit engine termination (`stop_all()`) on window close (`WindowCloseRequested`), maintaining live wallpaper playback continuously while closing the GUI.
  - Implemented seamless IPC socket re-adoption (`ipc_is_alive` and `ipc_get_pid`), allowing subsequent Aura sessions to detect and control active wallpapers without restarting them or causing display flicker.
  - Added graceful `quit` command signaling via IPC in `stop_output` and `stop_all` for adopted background processes.

## [1.3.2] - 2026-09-18

### Added
- **Dynamic Linux Distribution Detection**:
  - Added `detect_os_pretty_name()` parsing `/etc/os-release` to dynamically identify the host distribution (e.g. CachyOS, Arch Linux, Pop!_OS, Fedora, openSUSE).
  - The "Acerca de" (About) view now dynamically displays the active distribution alongside COSMIC Desktop (e.g. `Versión 1.3.2 • Arch Linux • COSMIC Desktop`).
  - Generalized Settings autostart label from Pop!_OS to COSMIC Desktop.
  - Updated CLI help header to reference COSMIC Desktop generically.

### Fixed
- **In-App Binary Update & Restart (`RestartApp`)**:
  - Fixed an issue where clicking "Restart" after updating Aura through the UI or toast notification would terminate Aura without reopening.
  - Replaced immediate synchronous spawning with a detached process launcher and a 400ms delay (`sh -c "sleep 0.4 && exec '$EXE' &"`), ensuring the dying parent fully unbinds from Wayland and relinquishes the D-Bus `io.github.antwny.aura` single-instance lock before the new instance initializes.
  - Added path resolution targeting `~/.local/bin/aura` and stripping Linux kernel `/proc/self/exe` ` (deleted)` suffixes when the binary was replaced in-place.

### Changed & Documentation
- **Universal COSMIC Desktop Ecosystem**:
  - Updated `README.md`, badges, and landing page documentation to emphasize universal COSMIC support across CachyOS, Arch Linux, Fedora, openSUSE, and Pop!_OS.
  - Added interactive tabs for Arch/CachyOS (`pacman`), Pop!_OS/Ubuntu (`apt`), Fedora (`dnf`), and openSUSE (`zypper`) in the web documentation prerequisites.
  - Refined `install.sh` platform checks to welcome all Linux distributions running COSMIC Desktop.

## [1.3.1] - 2026-09-17

### Added
- **Multi-Distribution COSMIC Support (CachyOS, Arch Linux, Fedora, openSUSE, Pop!_OS)**:
  - Added smart distro detection (`/etc/os-release` and package manager probes) across both installation scripts and the application runtime.
  - Tailored installation instructions for Arch Linux & CachyOS (`sudo pacman -S --needed mpv ffmpeg`), Debian/Pop!_OS (`sudo apt install -y libmpv2 ffmpeg`), Fedora (`sudo dnf install -y mpv-libs ffmpeg-free`), and openSUSE (`sudo zypper install -y mpv ffmpeg`).
  - Interactive dependency installer: when run interactively, the installer prompts to automatically install missing multimedia dependencies with `sudo`.
  - Unmissable end-of-terminal alert box ensuring dependency commands are never lost in terminal scrollback.
- **Engine Diagnostics & Health Verification**:
  - `aura status` now checks and reports true multimedia runtime health (verifying `mpvpaper` executable and `libmpv` shared library linkage).
  - Dynamic detection of active `mpvpaper` PIDs in `/proc` to report genuine playback vs paused vs stopped engine states.
  - Startup verification in `set_wallpaper`: catches early dynamic linker failures (exit 127) within 25ms and displays actionable distro-specific install hints via desktop toasts instead of silent failures.

### Fixed & Improved
- **Robust Process Management & Zombie Elimination**:
  - `stop_all` now cleans up running `mpvpaper` processes discovered via `/proc` and wipes orphaned IPC sockets even when invoked from separate CLI instances.
  - Added periodic process reaping in the 1-second tick loop (`reap_dead_processes`), automatically synchronizing tray and UI state if an external process terminates.
  - Added automatic IPC socket auto-discovery to `set_volume`, `set_mute`, and `set_scaling`.
- **CLI & Input Validation**:
  - `aura apply` and flag parsing now strictly validate that target files are regular files (`is_file()`) and match supported video/image extensions, preventing crashes or invalid arguments when given directories.
- **Atomic Configuration & Filesystem Reliability**:
  - `Config::save()` upgraded to atomic write via temporary file (`config.json.tmp.<pid>`) and POSIX `rename`, preventing 0-byte corrupt configs on unexpected reboot or termination.
  - Startup sweep to automatically clean orphaned partial download artifacts (`.tmp`, `.tmp.jpg`) in the online wallpapers directory.
- **Display Hotplug Resilience**:
  - Monitor hotplug handler now resets `selected_output` if a connected monitor is unplugged and terminates engine processes associated with disconnected displays.
- **Theme Palette & Explore Improvements**:
  - Low-vibrancy and monochrome/black-and-white wallpapers now produce an elegant neutral slate tone instead of default neon orange.
  - Online catalog pagination rolled back safely on network/API failure to avoid drifted page offsets.
  - Bundled `mpvpaper` binary is now automatically kept up to date alongside `aura` during `aura update`.

## [1.3.0] - 2026-09-13

### Added
- **Online Live Wallpapers Catalog (MotionBGS Integration)**:
  - Curated animated live wallpapers catalog in Explore tab with custom fire iconography (`fire-symbolic.svg`).
  - Full resolution selection for 4K and 1080p live wallpapers with accurate badge indicators.
  - Multi-category filters, live search, and paginated lazy-loading.
- **Interactive Download Progress & Dynamic Streaming**:
  - Live progress bar showing exact download percentage, MBs transferred, and total file size for both live and static wallpapers.
  - Interactive cancel button (`✕`) to cleanly abort ongoing downloads.
  - Dynamic chunk-idle timeout ensuring resilient streaming without premature aborts on slow networks.
  - Sanitized unique filenames avoiding overwrite collisions between downloaded live wallpapers.
- **Seamless MPV IPC Socket Engine Integration**:
  - Dedicated per-output UNIX IPC sockets (`/tmp/aura_mpv_<output>.sock`).
  - Zero-flicker live wallpaper switching via `loadfile <path> replace` without destroying Wayland surfaces or restarting `mpvpaper`.
  - Instant hot-adjustment for volume and audio mute in Settings via JSON IPC properties.
  - Dynamic aspect ratio and panscan control (`fit`, `fill`, `stretch`) in real-time.
- **Native Auto-Pause on Desktop Hidden**:
  - Direct integration with `mpvpaper -p` to automatically halt video rendering when windows obscure the desktop.
  - Delivers true 0% CPU and GPU utilization during active application use.
  - User-configurable toggle in Settings: *"Pausar automáticamente si el escritorio está oculto"* with bilingual ES/EN descriptions.

### Fixed
- **CLI `aura toggle-pause` Signal Bug**:
  - Fixed issue where repeated `aura toggle-pause` calls continuously sent `SIGSTOP` because `pkill -STOP` always returns exit code 0 in Linux.
  - Prioritized JSON IPC `cycle pause` commands with a robust fallback inspecting `/proc/<PID>/status` for `State: T (stopped)` before alternating between `SIGCONT` and `SIGSTOP`.
- **Explore Tab Applied Check Indicators**:
  - Fixed visual bug where all live wallpaper cards erroneously displayed the active checkmark.

## [1.2.2] - 2026-09-13

### Fixed
- **Library Toast Notification Stacking & Spam Elimination**:
  - Eliminated notification flooding when rapidly testing and applying multiple wallpapers in the Library.
  - Implemented immediate toast replacement (`notify_applied`): applying a new wallpaper cleans up previous toasts, ensuring at most one notification is active at any time.
  - Reduced toast display duration from 15 seconds (`Duration::Long`) to 5 seconds (`Duration::Short`).
  - Added duplicate check in `Message::ApplyWallpaper`: skips redundant re-applications, mpv respawns, and toast spam if the selected wallpaper is already running on that monitor.
  - Disabled redundant clicks on the active wallpaper card in the Library.
- **Automated mpvpaper Wayland Engine Compilation in CI**:
  - Configured GitHub Actions release workflow to install build tools (`meson`, `ninja`, `libwlroots-dev`) and compile `mpvpaper` from source on release, ensuring it is always bundled in release tarballs.

## [1.2.1] - 2026-09-12

### Added
- **Native 4K/UHD Static Wallpaper Rendering & Spline36 Filtering**:
  - Direct unconstrained GPU rendering of ultra-high-resolution wallpapers in `mpvpaper` without downscaling to 1440p.
  - Enhanced scaling pipeline with `--scale=spline36 --cscale=spline36 --dscale=mitchell` for sharp, artifact-free presentation on high-DPI displays.
  - High-definition Lanczos3 thumbnail generation preserving PNG transparency and 95% JPEG quality for safety caps on extreme (>8K) images.
  - Automatic thumbnail promotion in scanner: detects legacy low-resolution web previews (<16 KB) and regenerates crisp 480x270 Lanczos3 thumbnails directly from downloaded master files.
- **Reliable Update Lifecycle Notifications**:
  - Background update checks run seamlessly on boot and autostart (`--hidden`), preserving update status in memory.
  - Restoring or opening Aura from the system tray, dock, or D-Bus (`ShowMainWindow`) immediately triggers the update notification banner if an update is available.
  - Added smart 45-minute window reactivation check cooldown and a periodic 4-hour background subscription to notify about new releases during long uptime sessions.

### Changed
- **Streamlined Explore Download & Apply Flow**:
  - Consolidated redundant notifications into a single clear toast ("Fondo de pantalla aplicado") with an actionable `[ Mostrar en Archivos ]` button.
  - Prevented race conditions and wallpaper overriding when clicking multiple items in Explore (`pending_auto_apply_id`).
  - Disabled download interactions on in-progress items to prevent duplicate requests.

## [1.2.0] - 2026-09-11

### Added
- **COSMIC v2 Dynamic Desktop Accent Color Synchronization**:
  - Full compatibility with the modern COSMIC Desktop `v2` theming specification (`com.system76.CosmicTheme.Dark/v2`, `Light/v2`, and `Builder/v2`) alongside retrocompatible `v1` fallbacks.
  - Dynamic extraction of dominant accent colors from active video frames and static wallpapers.
  - Computes complete component palettes including `hover`, `pressed`, `disabled_border`, and WCAG relative luminance-based text contrast (`on` foreground in solid white or black).
  - Atomic temporary-file-and-rename writes ensuring instant inotify recognition by `cosmic-settings-daemon` (theme regenerates in ~40ms across GTK3, GTK4, Qt, and COSMIC apps).
  - Instant theme synchronization when toggling Auto-Theme or Auto-Dark in Settings, and across CLI commands (`aura apply`, `aura next`, `aura prev`).
- **DenverCoder1 Minimalistic Flat Art Catalog in Explore**:
  - Added new online wallpaper provider featuring the curated DenverCoder1 Minimalistic collection with multi-category filters (Landscape, City, Nature, Minimalistic, Anime, Abstract, Animals, Misc).
  - Parallel background thumbnail streaming via high-speed global CDN (`cdn.jsdelivr.net`).
  - Resilient Git LFS pointer validation with automatic raw fallback to prevent corrupt image downloads.

## [1.1.4] - 2026-09-11

### Fixed
- **0.5s Playback Freeze Resolved**:
  - Bound `mpvpaper` to Wayland layer `bottom` (`-l bottom`), ensuring it renders permanently above `cosmic-bg` (`background`) and preventing dynamic theme palette updates from occluding wallpaper playback.
- **Excluded Downloads & Generic Directories**:
  - Removed `~/Downloads`, `~/Descargas`, and root `~/Videos`/`~/Pictures` from default library directories, preventing UI hangs and excessive thumbnail generation on startup.
  - Automatically sanitizes existing configurations on load to strip legacy Downloads paths.
- **Opaque Letterbox on Ultrawide Displays**:
  - Configured mpv with `--background-color=#000000` so pillarboxed videos render solid black margins rather than leaking the desktop wallpaper on ultrawide monitors.
- **Auto-Rotation i18n Translations**:
  - Translated the playlist auto-rotation toggle and status notification messages for both English and Spanish.
- **Displays Tab Localization**:
  - Fixed untranslated "Destino activo" string displaying in Spanish when English is selected.
  - Fully localized scaling mode buttons ("Fit" / "Ajustar", "Fill" / "Rellenar", "Stretch" / "Estirar") and monitor status toasts.

### Changed
- **Optimized Library Header Button**:
  - Shortened "Abrir carpeta en el Gestor de Archivos" to "Abrir carpeta" (ES) and "Open folder" (EN) for a clean, non-cluttered header.

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
