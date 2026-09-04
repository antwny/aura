# Contributing to Aura 🌌

Thank you for your interest in contributing to **Aura**! 

Aura is designed to be a high-performance, rock-solid, and aesthetically native live wallpaper manager for the Pop!_OS COSMIC desktop environment. We welcome community contributions, bug reports, and feature proposals.

---

## 🛠️ Development Setup

### 1. Prerequisites
Ensure you have the required system and build dependencies installed:

```bash
# Pop!_OS 24.04 LTS / Ubuntu-based systems
sudo apt update
sudo apt install mpvpaper ffmpeg pkg-config libwayland-dev libxkbcommon-dev just
```

Ensure the Rust toolchain (1.80+) is installed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Building & Running Locally
Clone the repository and build:

```bash
git clone https://github.com/antwny/aura.git
cd aura

# Check code for compilation issues
just check

# Run Aura in debug mode
just run

# Build an optimized release binary
just build
```

---

## 📐 Code Style & Guidelines

1. **Idiomatic Rust**: Keep code clean, readable, and strictly typed. Prefer zero-cost abstractions and non-blocking asynchronous patterns (`tokio`).
2. **libcosmic Best Practices**:
   - Use native `libcosmic` widgets (`widget::container`, `widget::column`, `widget::row`, `widget::button`).
   - Follow standard COSMIC design guidelines: preserve clean monochrome symbolic icons in navigation bars, avoid hardcoded emojis next to system toggles, and respect system dark/light accent theming.
   - For dialogs and file selection, rely on the XDG Desktop Portal (`rfd` with `xdg-portal`).
3. **Format & Quality Check**:
   Before committing changes, format and verify the codebase:
   ```bash
   cargo fmt --check
   cargo check
   ```

---

## 🚀 Submitting Changes

1. **Fork & Branch**: Create a feature branch off `master`:
   ```bash
   git checkout -b feat/your-feature-name
   ```
2. **Commit Messages**: Write clear, descriptive commit messages following Conventional Commits (e.g. `feat: ...`, `fix: ...`, `refactor: ...`, `docs: ...`).
3. **Test**: Ensure the application launches smoothly, switches wallpapers across displays, and doesn't leak memory or background processes.
4. **Open a PR**: Submit a Pull Request describing your changes, with screenshots or terminal output if applicable.

---

## 💬 Community & Support

- **Bug Reports**: If you find an issue or unexpected behavior, open an issue on GitHub with steps to reproduce and system details (`cosmic-randr list`, GPU model, etc.).
- **License**: By contributing to Aura, you agree that your contributions will be licensed under the **GNU General Public License v3.0**.
