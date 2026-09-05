# Justfile for Aura - Live Wallpaper Manager for COSMIC Desktop

# Default recipe: build release binary
default: build

# Run Aura
run *args="":
    cargo run -- {{args}}

# Build release binary
build:
    cargo build --release

# Build debug binary
build-debug:
    cargo build

# Check code for errors and lints
check:
    cargo check

# Run automated tests
test:
    cargo test

# Validate desktop entry and AppStream metainfo
validate:
    @command -v desktop-file-validate >/dev/null 2>&1 && desktop-file-validate resources/io.github.antwny.aura.desktop || echo "desktop-file-validate not found"
    @command -v appstreamcli >/dev/null 2>&1 && appstreamcli validate --pedantic resources/io.github.antwny.aura.metainfo.xml || echo "appstreamcli not found"

# Format source code (if installed)
fmt:
    @command -v rustfmt >/dev/null 2>&1 && cargo fmt || echo "rustfmt not installed; skipping format"

# Run Clippy linter (if installed)
lint:
    @cargo clippy --help >/dev/null 2>&1 && cargo clippy -- -D warnings || echo "clippy not installed; skipping clippy"

# Clean build artifacts
clean:
    cargo clean

# Install Aura binary, desktop file, icon, and AppStream metainfo for current user
install: build
    install -d "${HOME}/.local/bin"
    install -m 755 target/release/aura "${HOME}/.local/bin/aura"
    install -d "${HOME}/.local/share/applications"
    install -m 644 resources/io.github.antwny.aura.desktop "${HOME}/.local/share/applications/io.github.antwny.aura.desktop"
    sed -i "s|^Exec=.*|Exec=${HOME}/.local/bin/aura|" "${HOME}/.local/share/applications/io.github.antwny.aura.desktop"
    install -d "${HOME}/.local/share/icons/hicolor/scalable/apps"
    install -m 644 resources/icons/hicolor/scalable/apps/io.github.antwny.aura.svg "${HOME}/.local/share/icons/hicolor/scalable/apps/io.github.antwny.aura.svg"
    rm -f "${HOME}/.local/share/icons/hicolor/*/apps/io.github.antwny.aura.png"
    @command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "${HOME}/.local/share/applications" 2>/dev/null || true
    @command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true
    install -d "${HOME}/.local/share/metainfo"
    install -m 644 resources/io.github.antwny.aura.metainfo.xml "${HOME}/.local/share/metainfo/io.github.antwny.aura.metainfo.xml"
    @echo "Aura successfully installed to ~/.local"

# Install system-wide to /usr/local (requires root privileges)
install-system: build
    install -d "/usr/local/bin"
    install -m 755 target/release/aura "/usr/local/bin/aura"
    install -d "/usr/local/share/applications"
    install -m 644 resources/io.github.antwny.aura.desktop "/usr/local/share/applications/io.github.antwny.aura.desktop"
    sed -i "s|^Exec=.*|Exec=/usr/local/bin/aura|" "/usr/local/share/applications/io.github.antwny.aura.desktop"
    install -d "/usr/local/share/icons/hicolor/scalable/apps"
    install -m 644 resources/icons/hicolor/scalable/apps/io.github.antwny.aura.svg "/usr/local/share/icons/hicolor/scalable/apps/io.github.antwny.aura.svg"
    rm -f "/usr/local/share/icons/hicolor/*/apps/io.github.antwny.aura.png"
    @command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "/usr/local/share/applications" 2>/dev/null || true
    @command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "/usr/local/share/icons/hicolor" 2>/dev/null || true
    install -d "/usr/local/share/metainfo"
    install -m 644 resources/io.github.antwny.aura.metainfo.xml "/usr/local/share/metainfo/io.github.antwny.aura.metainfo.xml"
    @echo "Aura successfully installed to /usr/local"

# Uninstall user installation
uninstall:
    rm -f "${HOME}/.local/bin/aura"
    rm -f "${HOME}/.local/share/applications/io.github.antwny.aura.desktop"
    rm -f "${HOME}/.local/share/icons/hicolor/scalable/apps/io.github.antwny.aura.svg"
    rm -f "${HOME}/.local/share/metainfo/io.github.antwny.aura.metainfo.xml"
    @command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "${HOME}/.local/share/applications" 2>/dev/null || true
    @command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true
    @echo "Aura uninstalled from ~/.local"

# Create release tarball archive (aura-vX.Y.Z-x86_64-linux.tar.gz)
package version="1.0.1": build
    @rm -rf "target/package/aura-v{{version}}-x86_64-linux"
    @mkdir -p "target/package/aura-v{{version}}-x86_64-linux"
    cp target/release/aura "target/package/aura-v{{version}}-x86_64-linux/"
    cp resources/io.github.antwny.aura.desktop "target/package/aura-v{{version}}-x86_64-linux/"
    cp resources/icons/hicolor/scalable/apps/io.github.antwny.aura.svg "target/package/aura-v{{version}}-x86_64-linux/"
    cp resources/io.github.antwny.aura.metainfo.xml "target/package/aura-v{{version}}-x86_64-linux/"
    cp LICENSE "target/package/aura-v{{version}}-x86_64-linux/"
    cp packaging/install.sh "target/package/aura-v{{version}}-x86_64-linux/install.sh"
    cp packaging/uninstall.sh "target/package/aura-v{{version}}-x86_64-linux/uninstall.sh"
    chmod +x "target/package/aura-v{{version}}-x86_64-linux/install.sh"
    chmod +x "target/package/aura-v{{version}}-x86_64-linux/uninstall.sh"
    tar -czf "target/package/aura-v{{version}}-x86_64-linux.tar.gz" -C "target/package" "aura-v{{version}}-x86_64-linux"
    @echo "Package created at target/package/aura-v{{version}}-x86_64-linux.tar.gz"
