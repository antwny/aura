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
5. En la descripción del PR, añade un resumen corto:
   ```markdown
   ### Application Summary
   * **Name**: Aura
   * **App ID**: `io.github.antwny.aura`
   * **Description**: Animated live wallpaper manager built natively in Rust for System76 COSMIC Desktop.
   * **License**: GPL-3.0-or-later
   * **Upstream**: https://github.com/antwny/aura
   ```

---

## Paso 5: Revisión y Publicación

* El bot de Flathub ejecutará el linter y la compilación de prueba.
* Si el bot o los revisores sugieren algún ajuste, haz commit directamente en tu rama `add-aura` y haz `git push`.
* Una vez aprobado y fusionado el PR:
  * Flathub creará automáticamente el repositorio `flathub/io.github.antwny.aura`.
  * Recibirás una invitación para tener acceso de mantenedor.
  * En 1–3 horas el paquete estará publicado en Flathub y aparecerá automáticamente en la **COSMIC Store** dentro de la sección **"COSMIC Apps"**.
