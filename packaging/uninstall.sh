#!/usr/bin/env bash
set -e

# Aura - Standalone Uninstaller
# Removes Aura from ~/.local or /usr/local

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
            echo "Usage: ./uninstall.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --user     Uninstall from ~/.local"
            echo "  --system   Uninstall from /usr/local (requires root / sudo)"
            echo "  --help     Show this help message"
            exit 0
            ;;
    esac
done

remove_from_prefix() {
    local P="$1"
    local REMOVED=0
    if [ -f "${P}/bin/aura" ]; then
        rm -f "${P}/bin/aura"
        REMOVED=1
    fi
    if [ -f "${P}/bin/mpvpaper" ]; then
        rm -f "${P}/bin/mpvpaper"
        REMOVED=1
    fi
    if [ -f "${P}/share/applications/io.github.antwny.aura.desktop" ]; then
        rm -f "${P}/share/applications/io.github.antwny.aura.desktop"
        REMOVED=1
    fi
    if [ -f "${P}/share/icons/hicolor/scalable/apps/io.github.antwny.aura.svg" ]; then
        rm -f "${P}/share/icons/hicolor/scalable/apps/io.github.antwny.aura.svg"
        REMOVED=1
    fi
    if [ -f "${P}/share/metainfo/io.github.antwny.aura.metainfo.xml" ]; then
        rm -f "${P}/share/metainfo/io.github.antwny.aura.metainfo.xml"
        REMOVED=1
    fi

    if [ "$REMOVED" -eq 1 ]; then
        command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "${P}/share/applications" 2>/dev/null || true
        command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "${P}/share/icons/hicolor" 2>/dev/null || true
        echo "Aura uninstalled from ${P}"
    fi
}

if [ "$MODE" = "system" ]; then
    remove_from_prefix "/usr/local"
elif [ "$MODE" = "user" ]; then
    remove_from_prefix "${HOME}/.local"
else
    # Auto: try both if running as root or current user
    if [ "$(id -u)" -eq 0 ]; then
        remove_from_prefix "/usr/local"
    fi
    remove_from_prefix "${HOME}/.local"
fi

echo "Aura uninstallation complete."
