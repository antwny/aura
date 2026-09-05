#!/usr/bin/env bash
set -e

# Aura - Standalone Installer
# Supports user installation (~/.local) or system-wide (/usr/local with sudo)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

MODE="auto"
for arg in "$@"; do
    case "$arg" in
        --system)
            MODE="system"
            ;;
        --user)
            MODE="user"
            ;;
        -h|--help)
            echo "Usage: ./install.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --user     Install to ~/.local (default if not root)"
            echo "  --system   Install to /usr/local (requires root / sudo)"
            echo "  --help     Show this help message"
            exit 0
            ;;
    esac
done

if [ "$MODE" = "system" ] || ([ "$MODE" = "auto" ] && [ "$(id -u)" -eq 0 ]); then
    PREFIX="/usr/local"
    INSTALL_TYPE="system-wide"
else
    PREFIX="${HOME}/.local"
    INSTALL_TYPE="user"
fi

BIN_DIR="${PREFIX}/bin"
DATA_DIR="${PREFIX}/share"
APPS_DIR="${DATA_DIR}/applications"
ICONS_DIR="${DATA_DIR}/icons/hicolor/scalable/apps"
META_DIR="${DATA_DIR}/metainfo"

echo "Installing Aura (${INSTALL_TYPE}) to ${PREFIX}..."

# Create target directories
install -d "${BIN_DIR}"
install -d "${APPS_DIR}"
install -d "${ICONS_DIR}"
install -d "${META_DIR}"

# Install binary
install -m 755 aura "${BIN_DIR}/aura"

# Install icon & metainfo
install -m 644 io.github.antwny.aura.svg "${ICONS_DIR}/io.github.antwny.aura.svg"
install -m 644 io.github.antwny.aura.metainfo.xml "${META_DIR}/io.github.antwny.aura.metainfo.xml"
rm -f "${DATA_DIR}/icons/hicolor"/*/apps/io.github.antwny.aura.png 2>/dev/null || true

# Install and configure desktop entry
install -m 644 io.github.antwny.aura.desktop "${APPS_DIR}/io.github.antwny.aura.desktop"

# Use absolute path for Exec and TryExec so desktop application launchers (COSMIC, GNOME, etc.)
# launch Aura directly without depending on session PATH.
sed -i "s|^Exec=.*|Exec=${BIN_DIR}/aura|" "${APPS_DIR}/io.github.antwny.aura.desktop"
if grep -q "^TryExec=" "${APPS_DIR}/io.github.antwny.aura.desktop"; then
    sed -i "s|^TryExec=.*|TryExec=${BIN_DIR}/aura|" "${APPS_DIR}/io.github.antwny.aura.desktop"
else
    sed -i "/^Exec=/i TryExec=${BIN_DIR}/aura" "${APPS_DIR}/io.github.antwny.aura.desktop"
fi

# Update desktop and icon databases
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "${APPS_DIR}" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "${DATA_DIR}/icons/hicolor" 2>/dev/null || true

# PATH configuration for user install
if [ "$INSTALL_TYPE" = "user" ]; then
    case ":$PATH:" in
        *":$HOME/.local/bin:"*)
            # Already present in current session PATH
            ;;
        *)
            MODIFIED=0
            if [ -f "$HOME/.bashrc" ] && ! grep -q '\.local/bin' "$HOME/.bashrc"; then
                echo '' >> "$HOME/.bashrc"
                echo '# Added by Aura installer' >> "$HOME/.bashrc"
                echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc"
                MODIFIED=1
            fi
            if [ -f "$HOME/.profile" ] && ! grep -q '\.local/bin' "$HOME/.profile"; then
                echo '' >> "$HOME/.profile"
                echo '# Added by Aura installer' >> "$HOME/.profile"
                echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.profile"
                MODIFIED=1
            fi
            if [ -f "$HOME/.zshrc" ] && ! grep -q '\.local/bin' "$HOME/.zshrc"; then
                echo '' >> "$HOME/.zshrc"
                echo '# Added by Aura installer' >> "$HOME/.zshrc"
                echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.zshrc"
                MODIFIED=1
            fi
            if [ "$MODIFIED" -eq 1 ]; then
                echo ""
                echo "Added ~/.local/bin to your PATH in ~/.bashrc / ~/.profile."
            fi
            echo ""
            echo "To use 'aura' in your current terminal session, run:"
            echo "  source ~/.bashrc"
            echo "or restart your terminal."
            ;;
    esac
fi

echo ""
echo "Aura installed successfully to ${BIN_DIR}/aura"

# Dependency check
echo ""
echo "Checking runtime dependencies:"
DEPS_MISSING=0
if command -v mpvpaper >/dev/null 2>&1; then
    echo "  [OK] mpvpaper (required to display live video wallpapers)"
else
    echo "  [MISSING] mpvpaper (required to display live video wallpapers)"
    DEPS_MISSING=1
fi

if command -v ffmpeg >/dev/null 2>&1; then
    echo "  [OK] ffmpeg (required to generate video thumbnails in library)"
else
    echo "  [MISSING] ffmpeg (required to generate video thumbnails in library)"
    DEPS_MISSING=1
fi

if [ "$DEPS_MISSING" -eq 1 ]; then
    echo ""
    echo "Tip: Aura will open and function normally, but to play live video wallpapers"
    echo "and generate video thumbnails, please install the missing dependencies:"
    echo "  sudo apt install mpvpaper ffmpeg"
fi
