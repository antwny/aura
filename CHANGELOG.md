# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
