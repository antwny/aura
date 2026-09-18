#!/usr/bin/env bash
set -e

# ==============================================================================
# Aura - Quick Web Installer
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/antwny/aura/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/antwny/aura/main/install.sh | bash -s -- --system
# ==============================================================================

# Terminal styling
BOLD="$(tput bold 2>/dev/null || printf '')"
GREEN="$(tput setaf 2 2>/dev/null || printf '')"
BLUE="$(tput setaf 4 2>/dev/null || printf '')"
YELLOW="$(tput setaf 3 2>/dev/null || printf '')"
RED="$(tput setaf 1 2>/dev/null || printf '')"
RESET="$(tput sgr0 2>/dev/null || printf '')"

echo "${BLUE}${BOLD}🌌 Aura — Instalador Rápido de Fondos Animados para COSMIC${RESET}"
echo "────────────────────────────────────────────────────────────"

# 1. Check OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" != "Linux" ]; then
    echo "${RED}Error: Aura solo es compatible con Linux (Pop!_OS / COSMIC Wayland).${RESET}"
    exit 1
fi

if [ "$ARCH" != "x86_64" ]; then
    echo "${RED}Error: La arquitectura actual ($ARCH) no está soportada. Aura requiere x86_64.${RESET}"
    exit 1
fi

# 2. Check curl or wget
if command -v curl >/dev/null 2>&1; then
    DOWNLOAD_CMD="curl -fsSL"
elif command -v wget >/dev/null 2>&1; then
    DOWNLOAD_CMD="wget -qO-"
else
    echo "${RED}Error: Se requiere 'curl' o 'wget' para descargar Aura.${RESET}"
    exit 1
fi

# Parse options
MODE="auto"
TARGET_VERSION=""
for arg in "$@"; do
    case "$arg" in
        --system)
            MODE="system"
            ;;
        --user)
            MODE="user"
            ;;
        --version=*)
            TARGET_VERSION="${arg#*=}"
            ;;
        -h|--help)
            echo "Uso: curl -fsSL https://raw.githubusercontent.com/antwny/aura/main/install.sh | bash [OPCIONES]"
            echo ""
            echo "Opciones:"
            echo "  --user          Instalar en ~/.local (por defecto si no es root)"
            echo "  --system        Instalar en /usr/local (requiere privilegios sudo)"
            echo "  --version=X.Y.Z Instalar una versión específica (ej. --version=1.3.0)"
            echo "  --help          Mostrar esta ayuda"
            exit 0
            ;;
    esac
done

# 3. Detect latest version if not specified
if [ -z "$TARGET_VERSION" ]; then
    echo "🔍 Buscando la última versión de Aura..."
    # Query redirect URL without hitting GitHub API rate limits
    LATEST_TAG=$(curl -fsSL -o /dev/null -w "%{url_effective}" https://github.com/antwny/aura/releases/latest 2>/dev/null | sed 's|.*/tag/||')

    if [ -z "$LATEST_TAG" ] || [ "$LATEST_TAG" = "latest" ]; then
        # Fallback to GitHub API if redirect didn't resolve tag
        LATEST_TAG=$(curl -fsSL https://api.github.com/repos/antwny/aura/releases/latest 2>/dev/null | grep '"tag_name":' | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/')
    fi

    if [ -z "$LATEST_TAG" ]; then
        LATEST_TAG="v1.3.1"
    fi
    VERSION="${LATEST_TAG#v}"
else
    VERSION="${TARGET_VERSION#v}"
    LATEST_TAG="v${VERSION}"
fi

echo "📦 Versión seleccionada: ${GREEN}v${VERSION}${RESET}"

# 4. Create temporary directory
TMP_DIR="$(mktemp -d -t aura-install-XXXXXX)"
trap 'rm -rf "$TMP_DIR"' EXIT INT TERM

TARBALL="aura-v${VERSION}-x86_64-linux.tar.gz"
DOWNLOAD_URL="https://github.com/antwny/aura/releases/download/${LATEST_TAG}/${TARBALL}"
CHECKSUM_URL="https://github.com/antwny/aura/releases/download/${LATEST_TAG}/${TARBALL}.sha256"

echo "⬇️  Descargando ${TARBALL}..."
if ! curl -fL --progress-bar "$DOWNLOAD_URL" -o "${TMP_DIR}/${TARBALL}"; then
    echo "${RED}Error: No se pudo descargar el paquete desde ${DOWNLOAD_URL}${RESET}"
    exit 1
fi

# Optional checksum verification
if curl -fsSL "$CHECKSUM_URL" -o "${TMP_DIR}/${TARBALL}.sha256" 2>/dev/null; then
    EXPECTED_SHA=$(awk '{print $1}' "${TMP_DIR}/${TARBALL}.sha256")
    ACTUAL_SHA=$(sha256sum "${TMP_DIR}/${TARBALL}" | awk '{print $1}')
    if [ "$EXPECTED_SHA" = "$ACTUAL_SHA" ]; then
        echo "🔒 Checksum SHA-256 verificado correctamente."
    else
        echo "${YELLOW}Advertencia: El checksum SHA-256 no coincide.${RESET}"
    fi
fi

# 5. Extract tarball
echo "📂 Descomprimiendo archivos..."
tar -xzf "${TMP_DIR}/${TARBALL}" -C "$TMP_DIR"

EXTRACTED_DIR="${TMP_DIR}/aura-v${VERSION}-x86_64-linux"
if [ ! -d "$EXTRACTED_DIR" ]; then
    EXTRACTED_DIR="$TMP_DIR"
fi

# 6. Execute internal installer
echo "⚙️  Instalando Aura y motor de video..."
DEPS_MISSING_FLAG=0
if [ -f "${EXTRACTED_DIR}/install.sh" ]; then
    chmod +x "${EXTRACTED_DIR}/install.sh"
    cd "$EXTRACTED_DIR"
    if [ "$MODE" = "system" ]; then
        ./install.sh --system
    else
        ./install.sh --user
    fi
    if [ -f "${EXTRACTED_DIR}/.deps_missing" ]; then
        DEPS_MISSING_FLAG=1
    fi
else
    echo "${RED}Error: El script de instalación interno no fue encontrado en el archivo descargado.${RESET}"
    exit 1
fi

# Distro detection for final banner
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
    elif command -v apt-get >/dev/null 2>&1 || [ "$id" = "pop" ] || [ "$id" = "ubuntu" ] || [ "$id" = "debian" ] || [ "$id" = "linuxmint" ] || [[ "$id_like" == *"debian"* ]] || [[ "$id_like" == *"ubuntu"* ]]; then
        DISTRO_NAME="Pop!_OS / Ubuntu / Debian"
        INSTALL_CMD="sudo apt install -y libmpv2 ffmpeg"
    elif command -v dnf >/dev/null 2>&1 || [ "$id" = "fedora" ] || [ "$id" = "nobara" ] || [[ "$id_like" == *"fedora"* ]]; then
        DISTRO_NAME="Fedora"
        INSTALL_CMD="sudo dnf install -y mpv-libs ffmpeg-free"
    elif command -v zypper >/dev/null 2>&1 || [ "$id" = "opensuse" ] || [ "$id" = "opensuse-tumbleweed" ] || [[ "$id_like" == *"suse"* ]]; then
        DISTRO_NAME="openSUSE"
        INSTALL_CMD="sudo zypper install -y mpv ffmpeg"
    elif command -v xbps-install >/dev/null 2>&1 || [ "$id" = "void" ]; then
        DISTRO_NAME="Void Linux"
        INSTALL_CMD="sudo xbps-install -S mpv ffmpeg"
    else
        DISTRO_NAME="Linux"
        INSTALL_CMD="sudo pacman -S mpv ffmpeg || sudo apt install libmpv2 ffmpeg"
    fi
}

detect_distro

# Check if mpvpaper runs without dynamic linker failure
INSTALLED_MPV="${HOME}/.local/bin/mpvpaper"
if [ ! -x "$INSTALLED_MPV" ]; then
    INSTALLED_MPV="/usr/local/bin/mpvpaper"
fi
if [ -x "$INSTALLED_MPV" ]; then
    if ! "$INSTALLED_MPV" -h >/dev/null 2>&1; then
        DEPS_MISSING_FLAG=1
    fi
fi
if ! command -v ffmpeg >/dev/null 2>&1; then
    DEPS_MISSING_FLAG=1
fi

echo ""
echo "${GREEN}${BOLD}✨ ¡Aura v${VERSION} instalado con éxito!${RESET}"
echo "────────────────────────────────────────────────────────────"
echo "🚀 Para abrir Aura:"
echo "  • Desde la terminal: ${BOLD}aura${RESET}"
echo "  • O búscalo como '${BOLD}Aura${RESET}' en el lanzador de aplicaciones de COSMIC"
echo ""
echo "🌌 Atajos y comandos útiles:"
echo "  • Ver estado actual:    ${BOLD}aura status${RESET}"
echo "  • Cambiar fondo:        ${BOLD}aura next${RESET}  (¡Asígnalo a Super + W en Ajustes de COSMIC!)"
echo "  • Pausar/Reanudar (0%): ${BOLD}aura toggle-pause${RESET}"

# If dependencies are missing, the VERY LAST output in the terminal must be this impossible-to-miss alert:
if [ "$DEPS_MISSING_FLAG" -eq 1 ]; then
    echo ""
    echo "${YELLOW}${BOLD}╔══════════════════════════════════════════════════════════════════════════╗${RESET}"
    echo "${YELLOW}${BOLD}║  ⚠️   ACCIÓN REQUERIDA PARA ACTIVAR FONDOS ANIMADOS (${DISTRO_NAME})${RESET}"
    echo "${YELLOW}${BOLD}╠══════════════════════════════════════════════════════════════════════════╣${RESET}"
    echo "${YELLOW}${BOLD}║${RESET} Aura necesita librerías de video que aún no están en tu sistema.         ${YELLOW}${BOLD}║${RESET}"
    echo "${YELLOW}${BOLD}║${RESET} Sin ellas, solo funcionarán fondos estáticos.                            ${YELLOW}${BOLD}║${RESET}"
    echo "${YELLOW}${BOLD}║${RESET}                                                                          ${YELLOW}${BOLD}║${RESET}"
    echo "${YELLOW}${BOLD}║${RESET} 👉 ${BOLD}Copia y ejecuta este comando para habilitarlas:${RESET}                      ${YELLOW}${BOLD}║${RESET}"
    echo "${YELLOW}${BOLD}║${RESET}                                                                          ${YELLOW}${BOLD}║${RESET}"
    echo "${YELLOW}${BOLD}║${RESET}    ${GREEN}${BOLD}${INSTALL_CMD}${RESET}"
    echo "${YELLOW}${BOLD}║${RESET}                                                                          ${YELLOW}${BOLD}║${RESET}"
    echo "${YELLOW}${BOLD}║${RESET} ¡Listo! Después de ejecutarlo, tus fondos cobrarán vida en COSMIC. 🌌     ${YELLOW}${BOLD}║${RESET}"
    echo "${YELLOW}${BOLD}╚══════════════════════════════════════════════════════════════════════════╝${RESET}"
fi
echo ""
