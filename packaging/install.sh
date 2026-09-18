#!/usr/bin/env bash
set -e

# Aura - Standalone Installer
# Supports user installation (~/.local) or system-wide (/usr/local with sudo)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd)"
if [ -n "$SCRIPT_DIR" ] && [ -d "$SCRIPT_DIR" ]; then
    cd "$SCRIPT_DIR"
fi

# If invoked via curl or without pre-extracted binaries, delegate to web installer
if [ ! -f "aura" ]; then
    if command -v curl >/dev/null 2>&1; then
        exec bash -c "$(curl -fsSL https://raw.githubusercontent.com/antwny/aura/main/install.sh)" -- "$@"
    elif command -v wget >/dev/null 2>&1; then
        exec bash -c "$(wget -qO- https://raw.githubusercontent.com/antwny/aura/main/install.sh)" -- "$@"
    fi
fi

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

# Install bundled mpvpaper if provided
if [ -f "mpvpaper" ]; then
    install -m 755 mpvpaper "${BIN_DIR}/mpvpaper"
fi

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

# If autostart entry exists from a previous session, ensure it points to the installed binary
AUTOSTART_FILE="${HOME}/.config/autostart/io.github.antwny.aura.desktop"
if [ -f "${AUTOSTART_FILE}" ]; then
    sed -i "s|^Exec=.*|Exec=${BIN_DIR}/aura --hidden|" "${AUTOSTART_FILE}"
    if grep -q "^TryExec=" "${AUTOSTART_FILE}"; then
        sed -i "s|^TryExec=.*|TryExec=${BIN_DIR}/aura|" "${AUTOSTART_FILE}"
    else
        sed -i "/^Exec=/i TryExec=${BIN_DIR}/aura" "${AUTOSTART_FILE}"
    fi
fi

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

# Distro detection
detect_distro() {
    local id=""
    local id_like=""
    if [ -f /etc/os-release ]; then
        # shellcheck disable=SC1091
        . /etc/os-release
        id="${ID:-}"
        id_like="${ID_LIKE:-}"
    fi

    if command -v pacman >/dev/null 2>&1 || [ "$id" = "cachyos" ] || [ "$id" = "arch" ] || [ "$id" = "manjaro" ] || [ "$id" = "endeavouros" ] || [[ "$id_like" == *"arch"* ]]; then
        DISTRO_NAME="Arch / CachyOS"
        INSTALL_CMD="sudo pacman -S --needed mpv ffmpeg"
        PKG_MGR="pacman"
    elif command -v apt-get >/dev/null 2>&1 || [ "$id" = "pop" ] || [ "$id" = "ubuntu" ] || [ "$id" = "debian" ] || [ "$id" = "linuxmint" ] || [[ "$id_like" == *"debian"* ]] || [[ "$id_like" == *"ubuntu"* ]]; then
        DISTRO_NAME="Pop!_OS / Ubuntu / Debian"
        INSTALL_CMD="sudo apt install -y libmpv2 ffmpeg"
        PKG_MGR="apt"
    elif command -v dnf >/dev/null 2>&1 || [ "$id" = "fedora" ] || [ "$id" = "nobara" ] || [[ "$id_like" == *"fedora"* ]]; then
        DISTRO_NAME="Fedora"
        INSTALL_CMD="sudo dnf install -y mpv-libs ffmpeg-free"
        PKG_MGR="dnf"
    elif command -v zypper >/dev/null 2>&1 || [ "$id" = "opensuse" ] || [ "$id" = "opensuse-tumbleweed" ] || [[ "$id_like" == *"suse"* ]]; then
        DISTRO_NAME="openSUSE"
        INSTALL_CMD="sudo zypper install -y mpv ffmpeg"
        PKG_MGR="zypper"
    elif command -v xbps-install >/dev/null 2>&1 || [ "$id" = "void" ]; then
        DISTRO_NAME="Void Linux"
        INSTALL_CMD="sudo xbps-install -S mpv ffmpeg"
        PKG_MGR="xbps"
    else
        DISTRO_NAME="Linux"
        INSTALL_CMD="sudo pacman -S mpv ffmpeg || sudo apt install libmpv2 ffmpeg"
        PKG_MGR="unknown"
    fi
}

detect_distro

# Dependency check
echo ""
echo "Verificando dependencias multimedia (${DISTRO_NAME}):"
DEPS_MISSING=0

# Verify mpvpaper and dynamic linking to libmpv
MPV_TARGET="${BIN_DIR}/mpvpaper"
if [ ! -x "$MPV_TARGET" ]; then
    MPV_TARGET="$(command -v mpvpaper 2>/dev/null || printf '')"
fi

if [ -n "$MPV_TARGET" ] && [ -x "$MPV_TARGET" ]; then
    if "$MPV_TARGET" -h >/dev/null 2>&1; then
        echo "  [OK] mpvpaper & libmpv (reproducción de fondos animados lista)"
    else
        echo "  [FALTA] libmpv / códecs de video (mpvpaper no pudo cargar librerías compartidas)"
        DEPS_MISSING=1
    fi
else
    echo "  [FALTA] mpvpaper (binario no disponible)"
    DEPS_MISSING=1
fi

if command -v ffmpeg >/dev/null 2>&1; then
    echo "  [OK] ffmpeg (generación de miniaturas de video)"
else
    echo "  [FALTA] ffmpeg (requerido para miniaturas en la biblioteca)"
    DEPS_MISSING=1
fi

# If interactive and dependencies are missing, offer to install them now
if [ "$DEPS_MISSING" -eq 1 ] && [ -t 0 ] && [ -t 1 ]; then
    echo ""
    echo "💡 Se detectaron dependencias multimedia pendientes para tu distribución (${DISTRO_NAME})."
    read -r -p "¿Deseas instalarlas automáticamente con sudo ahora? [S/n] " prompt_ans </dev/tty || prompt_ans="n"
    case "$prompt_ans" in
        [sS]|[yY]|"")
            echo "Ejecutando: ${INSTALL_CMD}"
            if eval "$INSTALL_CMD"; then
                echo "✅ Dependencias instaladas exitosamente."
                DEPS_MISSING=0
            else
                echo "⚠️  No se pudo completar la instalación automática."
            fi
            ;;
        *)
            echo "Instalación de dependencias omitida por el usuario."
            ;;
    esac
fi

if [ "$DEPS_MISSING" -eq 1 ]; then
    echo ""
    echo "════════════════════════════════════════════════════════════════════════"
    echo "⚠️   ACCIÓN REQUERIDA PARA ACTIVAR FONDOS ANIMADOS (${DISTRO_NAME})"
    echo "────────────────────────────────────────────────────────────────────────"
    echo "Aura se instaló correctamente, pero tu sistema aún necesita las"
    echo "librerías de video para reproducir fondos animados."
    echo ""
    echo "👉 Ejecuta este comando en tu terminal:"
    echo "   ${INSTALL_CMD}"
    echo "════════════════════════════════════════════════════════════════════════"
    # Create marker file for parent install.sh if invoked through web installer
    touch "${SCRIPT_DIR}/.deps_missing" 2>/dev/null || true
else
    rm -f "${SCRIPT_DIR}/.deps_missing" 2>/dev/null || true
fi
