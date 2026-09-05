# 🚀 Guía de Envío de Aura a Flathub & COSMIC Store

Este directorio contiene los archivos oficiales requeridos por **Flathub** para publicar **Aura**:

* **`io.github.antwny.aura.yml`**: Manifiesto de compilación Flatpak configurado con `com.system76.Cosmic.BaseApp`.
* **`cargo-sources.json`**: Lista de dependencias offline de Cargo con sus checksums criptográficos.
* **`flathub.json`**: Restricción de arquitectura a `x86_64`.

---

## Paso 1: Subir las capturas y metadatos a GitHub

Para que los servidores de Flathub puedan validar las URLs de las capturas de pantalla, haz commit y push de las capturas y el archivo `metainfo.xml` actualizado:

```bash
# 1. Añadir y confirmar los cambios en Aura
git add Cargo.toml Cargo.lock CHANGELOG.md README.md docs/screenshots/ resources/ packaging/
git commit -m "chore(release): release v1.0.0 for official Flathub and COSMIC Store debut"

# 2. Crear el tag v1.0.0 oficial
git tag -a v1.0.0 -m "Release v1.0.0: Official production release for Flathub & COSMIC Store"

# 3. Subir a GitHub
git push origin main
git push origin v1.0.0
```

---

## Paso 2: Clonar Flathub y crear la rama de envío

Flathub gestiona las solicitudes de nuevas aplicaciones a través de la rama **`new-pr`** de su repositorio:

```bash
# 1. Haz un fork de https://github.com/flathub/flathub en tu cuenta de GitHub
#    (¡IMPORTANTE! Desmarca la casilla "Copy the master branch only").

# 2. Clona tu fork apuntando a la rama new-pr
git clone --branch=new-pr git@github.com:antwny/flathub.git ~/flathub-submission
cd ~/flathub-submission

# 3. Crea una rama para Aura a partir de new-pr
git checkout -b add-aura new-pr
```

---

## Paso 3: Copiar los 3 archivos de empaquetado a la raíz de Flathub

```bash
# Copia los tres archivos a la raíz de tu clon de flathub
cp /home/antwny/aura/packaging/flatpak/io.github.antwny.aura.yml .
cp /home/antwny/aura/packaging/flatpak/cargo-sources.json .
cp /home/antwny/aura/packaging/flatpak/flathub.json .

# Haz commit y sube tu rama
git add io.github.antwny.aura.yml cargo-sources.json flathub.json
git commit -m "Add io.github.antwny.aura"
git push -u origin add-aura
```

---

## Paso 4: Abrir el Pull Request

1. Ve a `https://github.com/flathub/flathub`.
2. Verás el botón para abrir un Pull Request desde tu rama `add-aura`.
3. **Verifica que la rama base destino sea `new-pr`** (NUNCA `master`).
4. Título del PR:
   ```text
   Add io.github.antwny.aura
   ```
5. En la descripción del PR, copia y pega este texto exacto:

```markdown
### Application Summary
* **Name**: Aura
* **App ID**: `io.github.antwny.aura`
* **Description**: Next-generation animated live wallpaper manager built natively in Rust for System76 COSMIC Desktop.
* **License**: GPL-3.0-or-later
* **Upstream**: https://github.com/antwny/aura

### Permissions Justification
* `--filesystem=xdg-run/wayland-*`: Required for Wayland Layer-Shell access to render animated wallpapers directly behind desktop surfaces via `mpvpaper`.
* `--device=dri`: Required for hardware-accelerated video decoding (VAAPI/Vulkan) and UI rendering (wgpu).
* `--filesystem=xdg-config/cosmic:rw`: Aura features an automated theming engine that extracts color palettes from active wallpapers and synchronizes accent colors with the COSMIC desktop shell (`~/.config/cosmic`).
* `--filesystem=xdg-config/autostart:create`: Allows users to toggle "Start at login" from within settings, creating an autostart entry for the headless wallpaper daemon.
* `--talk-name=com.system76.CosmicSettingsDaemon.*`: D-Bus interface for live color palette and settings synchronization with the COSMIC desktop environment.
* `--talk-name=org.kde.StatusNotifierWatcher`: Required for system tray indicator integration via `ksni` StatusNotifierItem.
* `--filesystem=xdg-pictures:rw` / `xdg-videos:rw` / `xdg-download:rw`: Local file scanning and management for user wallpapers.
* `--share=network`: Downloading wallpapers from curated online catalogs (Bing UHD & Wallhaven).
```

---

## Paso 5: Revisión y Publicación

* El bot de Flathub ejecutará automáticamente el linter y la compilación de prueba en sus servidores.
* Si los revisores aprueban las justificaciones de permisos (especialmente las extensiones de escritorio COSMIC), fusionarán el Pull Request.
* Una vez fusionado:
  * Flathub creará automáticamente el repositorio `flathub/io.github.antwny.aura`.
  * Recibirás una invitación por correo/notificación de GitHub para ser mantenedor oficial del repositorio de Flathub.
  * En pocas horas la app se publicará en Flathub y aparecerá automáticamente en la **COSMIC Store** (App Library de Pop!_OS) dentro de la sección **"COSMIC Apps"**.
