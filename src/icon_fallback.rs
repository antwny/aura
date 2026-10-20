use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const THEME_INDEX: &[u8] = b"[Icon Theme]\n\
Name=Hicolor\n\
Comment=Aura bundled symbolic icons\n\
Directories=scalable/actions\n\
\n\
[scalable/actions]\n\
Size=16\n\
Type=Scalable\n\
MinSize=8\n\
MaxSize=512\n\
Context=Actions\n";

const ICONS: &[(&str, &[u8])] = &[
    (
        "navbar-closed-symbolic.svg",
        include_bytes!("../resources/icons/navbar-closed-symbolic.svg"),
    ),
    (
        "navbar-open-symbolic.svg",
        include_bytes!("../resources/icons/navbar-open-symbolic.svg"),
    ),
    (
        "window-close-symbolic.svg",
        include_bytes!("../resources/icons/window-close-symbolic.svg"),
    ),
    (
        "window-maximize-symbolic.svg",
        include_bytes!("../resources/icons/window-maximize-symbolic.svg"),
    ),
    (
        "window-minimize-symbolic.svg",
        include_bytes!("../resources/icons/window-minimize-symbolic.svg"),
    ),
    (
        "window-restore-symbolic.svg",
        include_bytes!("../resources/icons/window-restore-symbolic.svg"),
    ),
];

pub fn install() -> bool {
    for root in overlay_roots() {
        let Ok(data_root) = write_overlay(&root) else {
            continue;
        };
        let Some(data_dirs) =
            merged_data_dirs(std::env::var_os("XDG_DATA_DIRS").as_deref(), &data_root)
        else {
            continue;
        };
        std::env::set_var("XDG_DATA_DIRS", data_dirs);
        return true;
    }

    false
}

fn overlay_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    for var in ["XDG_RUNTIME_DIR", "XDG_CACHE_HOME"] {
        if let Some(base) = nonempty_var(var) {
            push_unique(&mut roots, base.join("aura/icon-fallback"));
        }
    }

    if let Some(home) = nonempty_var("HOME") {
        push_unique(&mut roots, home.join(".cache/aura/icon-fallback"));
    }

    roots
}

fn nonempty_var(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}

fn write_overlay(root: &Path) -> io::Result<PathBuf> {
    let data_root = root.join("xdg");
    let theme_dir = data_root.join("icons/hicolor");
    let actions_dir = theme_dir.join("scalable/actions");

    fs::create_dir_all(&actions_dir)?;
    write_if_changed(&theme_dir.join("index.theme"), THEME_INDEX)?;

    for (name, contents) in ICONS {
        write_if_changed(&actions_dir.join(name), contents)?;
    }

    Ok(data_root)
}

fn write_if_changed(path: &Path, contents: &[u8]) -> io::Result<()> {
    if fs::read(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }

    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&tmp, contents)?;

    if let Err(error) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(error);
    }

    Ok(())
}

fn merged_data_dirs(existing: Option<&OsStr>, overlay: &Path) -> Option<OsString> {
    let mut dirs = vec![overlay.to_path_buf()];
    let existing: Vec<PathBuf> = existing
        .filter(|value| !value.is_empty())
        .map(|value| std::env::split_paths(value).collect())
        .unwrap_or_else(|| {
            vec![
                PathBuf::from("/usr/local/share"),
                PathBuf::from("/usr/share"),
            ]
        });

    for dir in existing {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }

    std::env::join_paths(dirs).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_complete_hicolor_overlay() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let data_root = write_overlay(temp.path()).expect("write overlay");
        let theme_dir = data_root.join("icons/hicolor");
        let actions_dir = theme_dir.join("scalable/actions");

        assert_eq!(
            fs::read(theme_dir.join("index.theme")).expect("read index"),
            THEME_INDEX
        );
        for (name, contents) in ICONS {
            assert_eq!(
                fs::read(actions_dir.join(name)).expect("read icon"),
                *contents
            );
        }
    }

    #[test]
    fn rewrites_changed_icons_and_keeps_directory_clean() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let data_root = write_overlay(temp.path()).expect("write overlay");
        let icon_path = data_root
            .join("icons/hicolor/scalable/actions/navbar-open-symbolic.svg");

        fs::write(&icon_path, b"stale").expect("replace icon");
        write_overlay(temp.path()).expect("rewrite overlay");

        assert_eq!(
            fs::read(&icon_path).expect("read icon"),
            include_bytes!("../resources/icons/navbar-open-symbolic.svg")
        );
        let names: Vec<_> = fs::read_dir(icon_path.parent().expect("actions dir"))
            .expect("read actions")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect();
        assert_eq!(names.len(), ICONS.len());
        assert!(names.iter().all(|name| !name.to_string_lossy().contains(".tmp-")));
    }

    #[test]
    fn prepends_overlay_and_preserves_existing_data_dirs() {
        let overlay = Path::new("/run/user/1000/aura/icon-fallback/xdg");
        let merged = merged_data_dirs(Some(OsStr::new("/usr/share:/opt/share")), overlay)
            .expect("merge data dirs");
        let dirs: Vec<_> = std::env::split_paths(&merged).collect();

        assert_eq!(
            dirs,
            vec![
                overlay.to_path_buf(),
                PathBuf::from("/usr/share"),
                PathBuf::from("/opt/share"),
            ]
        );
    }

    #[test]
    fn uses_default_data_dirs_when_unset() {
        let overlay = Path::new("/run/user/1000/aura/icon-fallback/xdg");
        let merged = merged_data_dirs(None, overlay).expect("merge data dirs");
        let dirs: Vec<_> = std::env::split_paths(&merged).collect();

        assert_eq!(
            dirs,
            vec![
                overlay.to_path_buf(),
                PathBuf::from("/usr/local/share"),
                PathBuf::from("/usr/share"),
            ]
        );
    }

    #[test]
    fn does_not_duplicate_overlay() {
        let overlay = Path::new("/run/user/1000/aura/icon-fallback/xdg");
        let existing = OsStr::new("/usr/share:/run/user/1000/aura/icon-fallback/xdg");
        let merged = merged_data_dirs(Some(existing), overlay).expect("merge data dirs");
        let dirs: Vec<_> = std::env::split_paths(&merged).collect();

        assert_eq!(
            dirs,
            vec![overlay.to_path_buf(), PathBuf::from("/usr/share")]
        );
    }
}
