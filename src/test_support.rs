use std::sync::{Once, OnceLock};

static TEST_DIR: OnceLock<tempfile::TempDir> = OnceLock::new();
static ENV_INIT: Once = Once::new();

/// Points every test at an isolated temporary HOME/XDG tree so unit tests can
/// never touch the real user configuration, cache, or wallpaper folders.
pub fn init() {
    let dir = TEST_DIR.get_or_init(|| {
        tempfile::Builder::new()
            .prefix("aura-tests-")
            .tempdir()
            .expect("create temporary test directory")
    });

    ENV_INIT.call_once(|| {
        let root = dir.path();
        let home = root.join("home");
        let config = home.join(".config");
        let data = home.join(".local/share");
        let cache = home.join(".cache");
        let runtime = root.join("runtime");

        for path in [&home, &config, &data, &cache, &runtime] {
            std::fs::create_dir_all(path).expect("create test XDG directory");
        }
        std::fs::create_dir_all(home.join("Pictures")).expect("create test Pictures directory");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&runtime, std::fs::Permissions::from_mode(0o700));
        }

        std::env::set_var("HOME", &home);
        std::env::set_var("XDG_CONFIG_HOME", &config);
        std::env::set_var("XDG_DATA_HOME", &data);
        std::env::set_var("XDG_CACHE_HOME", &cache);
        std::env::set_var("XDG_RUNTIME_DIR", &runtime);
    });
}
