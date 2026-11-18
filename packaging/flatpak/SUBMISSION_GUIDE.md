# 🚀 Guía de Envío de Aura a Flathub & COSMIC Store

Este directorio contiene los archivos oficiales requeridos por **Flathub** para publicar **Aura**:

* **`io.github.antwny.aura.yml`**: Manifiesto de compilación Flatpak configurado con `com.system76.Cosmic.BaseApp` (runtime 25.08) apuntando al tag `v1.10.0`.
* **`cargo-sources.json`**: Dependencias vendored offline de Cargo generadas para `Cargo.lock` v1.10.0 con sus checksums criptográficos.
* **`flathub.json`**: Restricción de arquitectura a `x86_64`.

---

## Estado Actual de Aura (v1.10.0)

- **Versión**: 1.10.0
- **Historial de desarrollo**: Más de 2 meses de desarrollo continuo y 10 versiones estables publicadas (v1.0.0 a v1.10.0), cumpliendo completamente la política de *Development History* de Flathub.
- **Validación AppStream**: `appstreamcli validate --no-net` y `flatpak-builder-lint appstream` superados al 100%.
- **Validación Desktop**: `desktop-file-validate` superado sin advertencias.
- **Iconos**: Se incluyen el icono de la aplicación (`io.github.antwny.aura.svg`) y todos los iconos simbólicos de interfaz (`navbar-open`, `navbar-closed`, `fire`, `window-*`).

---

## Paso 1: Confirmar cambios en el repositorio Aura

Asegúrate de confirmar los archivos de empaquetado actualizados en la rama principal de Aura:

```bash
cd /home/antwny/aura
git add packaging/flatpak/
git commit -m "chore(flatpak): prepare manifest and cargo-sources for v1.10.0 Flathub submission"
git push origin main
```

---

## Paso 2: Preparar la rama en el fork de Flathub

Flathub gestiona las solicitudes de nuevas aplicaciones a través de la rama **`new-pr`** de su repositorio:

```bash
# 1. Clonar tu fork de flathub apuntando a new-pr
git clone --branch=new-pr https://github.com/antwny/flathub.git ~/flathub-submission
cd ~/flathub-submission

# 2. Sincronizar new-pr con upstream (por si acaso han pasado semanas)
git remote add upstream https://github.com/flathub/flathub.git
git fetch upstream new-pr
git reset --hard upstream/new-pr

# 3. Crear una nueva rama limpia a partir de new-pr
git checkout -b add-aura-v1.10
```

---

## Paso 3: Copiar los 3 archivos a la raíz del repositorio

```bash
cp /home/antwny/aura/packaging/flatpak/io.github.antwny.aura.yml .
cp /home/antwny/aura/packaging/flatpak/cargo-sources.json .
cp /home/antwny/aura/packaging/flatpak/flathub.json .

# Verificar y commitear
git add io.github.antwny.aura.yml cargo-sources.json flathub.json
git commit -m "Add io.github.antwny.aura"
git push -u origin add-aura-v1.10
```

---

## Paso 4: Abrir el Pull Request en Flathub

1. Ve a **`https://github.com/flathub/flathub`**.
2. Haz clic en **"Compare & pull request"** para tu rama `add-aura-v1.10`.
3. **⚠️ IMPORTANTE:** Asegúrate de que la rama base destino sea **`new-pr`** (NUNCA `master`).
4. **Título del Pull Request**:
   ```text
   Add io.github.antwny.aura
   ```
5. **Cuerpo del Pull Request** (copia y pega la plantilla oficial completada):

```markdown
<!-- ⚠️⚠️  Submission pull request MUST be made against the `new-pr` **base branch** ⚠️⚠️  -->

### Please confirm your submission meets all the criteria

- [X] Please describe the application briefly.
  Aura is a high-performance animated live wallpaper manager built natively in Rust for the System76 COSMIC Desktop and modern Wayland desktops (with GNOME Hanabi and KDE Plasma 6 support). Features 60 FPS GPU-accelerated video rendering via mpvpaper, Wayland layer-shell protocol integration, online wallpaper catalogs (Bing UHD, Wallhaven, and LiveWallpapers4Free), dynamic COSMIC auto-theming accent synchronization, and intelligent battery/fullscreen throttling.
- [X] Please attach a video showcasing the application on Linux using the Flatpak.

https://github.com/user-attachments/assets/f5578555-0246-45e4-aa0e-11de57aebb50

- [X] The Flatpak ID follows all the rules listed in the [Application ID requirements][appid].
- [X] I have read and followed all the [Submission requirements][reqs] and the [Submission guide][reqs2] and I agree to them.
  - [X] The application has a meaningful development history, evidence of real-world use, and a clear commitment to ongoing maintenance, as required by the [development history requirements][history].
    (Active development over 2+ months with 10 releases: v1.0.0 through v1.10.0, automated CI/CD releases, active community feedback).
  - [X] I have disclosed any AI-generated material included in the application or its Flathub packaging, as required by the [Generative AI policy][ai]. **Affected parts and approximate extent:** AI pair programming assistance was used for boilerplate scaffolding, multilingual translations (i18n), and unit tests; all code, architectures, and assets were reviewed, authored, and are maintained by the author.
  - [X] I have not used AI tools or agents to generate or automate this submission pull request or its review interactions.
- [X] I am the author/developer of the project.

### Permissions Justification
* `--filesystem=xdg-run/wayland-*`: Required for direct Wayland Layer-Shell access to render animated wallpapers behind desktop surfaces via `mpvpaper`.
* `--device=dri`: Required for hardware-accelerated video decoding (VAAPI/Vulkan) and UI rendering (`wgpu`).
* `--filesystem=xdg-config/cosmic:rw`: Aura features an automated theming engine that extracts color palettes from active wallpapers and synchronizes accent colors with the COSMIC desktop shell (`~/.config/cosmic`).
* `--filesystem=xdg-config/autostart:create`: Allows users to toggle "Start at login" from within settings, creating an autostart entry on the host for the wallpaper daemon.
* `--talk-name=com.system76.CosmicSettingsDaemon.*`: D-Bus interface for live color palette and settings synchronization with the COSMIC desktop environment.
* `--talk-name=org.kde.StatusNotifierWatcher`: Required for system tray indicator integration via `ksni` StatusNotifierItem.
* `--talk-name=org.kde.plasmashell`: Required for applying wallpapers to the KDE Plasma desktop shell via D-Bus PlasmaShell scripting interface.
* `--talk-name=io.github.jeffshee.HanabiRenderer`: Required for video wallpaper playback synchronization on GNOME via the Hanabi extension.
* `--filesystem=xdg-pictures:rw` / `xdg-videos:rw` / `xdg-download:rw`: Local file scanning and management for user wallpapers.
* `--share=network`: Downloading wallpapers from curated online catalogs (Bing UHD, Wallhaven, LiveWallpapers4Free).

[appid]: https://docs.flathub.org/docs/for-app-authors/requirements#application-id
[reqs]: https://docs.flathub.org/docs/for-app-authors/requirements
[reqs2]: https://docs.flathub.org/docs/for-app-authors/submission
[history]: https://docs.flathub.org/docs/for-app-authors/requirements#insufficient-development-history
[ai]: https://docs.flathub.org/docs/for-app-authors/requirements#generative-ai-policy
```

---

## Paso 5: Revisión y Publicación

1. El bot de Flathub ejecutará el build automático de prueba en x86_64.
2. Un revisor humano revisará las justificaciones de permisos (`xdg-config/cosmic`, `autostart:create`, `wayland-*`, `plasmashell`).
3. Responde a cualquier pregunta de los revisores directamente en GitHub.
4. Una vez aprobado y fusionado el PR:
   - Flathub creará el repositorio oficial `flathub/io.github.antwny.aura`.
   - Recibirás una invitación para ser co-mantenedor.
   - La app se compilará y aparecerá en **Flathub** y en la **COSMIC Store** (sección COSMIC Apps de Pop!_OS).
