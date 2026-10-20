use ksni::menu::StandardItem;
use ksni::{Handle, MenuItem, ToolTip, Tray, TrayMethods};
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex as TokioMutex;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    Activate,
    ShowApp,
    OpenSwitcher,
    TogglePause,
    NextWallpaper,
    StopWallpaper,
    QuitApp,
}

#[derive(Clone)]
pub struct AuraTray {
    pub tx: UnboundedSender<TrayAction>,
    pub current_name: Arc<Mutex<String>>,
    pub is_paused: Arc<Mutex<bool>>,
    pub has_wallpaper: Arc<Mutex<bool>>,
    pub language: Arc<Mutex<crate::i18n::Language>>,
}

impl Tray for AuraTray {
    fn id(&self) -> String {
        "io.github.antwny.aura".into()
    }

    fn title(&self) -> String {
        "Aura".into()
    }

    fn icon_name(&self) -> String {
        "io.github.antwny.aura".into()
    }

    fn tool_tip(&self) -> ToolTip {
        let lang = *lock(&self.language);
        let name = lock(&self.current_name).clone();
        let desc = if name.is_empty() {
            lang.tray_tooltip_idle().to_string()
        } else {
            lang.tray_tooltip_playing(&name)
        };
        ToolTip {
            title: "Aura • Live Wallpaper".into(),
            description: desc,
            icon_name: "io.github.antwny.aura".into(),
            icon_pixmap: Vec::new(),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayAction::Activate);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let lang = *lock(&self.language);
        let is_paused = *lock(&self.is_paused);
        let has_wall = *lock(&self.has_wallpaper);

        vec![
            StandardItem {
                label: lang.tray_switcher().into(),
                activate: Box::new(|tray: &mut AuraTray| {
                    let _ = tray.tx.send(TrayAction::OpenSwitcher);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: lang.tray_open().into(),
                activate: Box::new(|tray: &mut AuraTray| {
                    let _ = tray.tx.send(TrayAction::ShowApp);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: if is_paused {
                    lang.tray_resume().into()
                } else {
                    lang.tray_pause().into()
                },
                enabled: has_wall,
                activate: Box::new(|tray: &mut AuraTray| {
                    let _ = tray.tx.send(TrayAction::TogglePause);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: lang.tray_next().into(),
                enabled: has_wall,
                activate: Box::new(|tray: &mut AuraTray| {
                    let _ = tray.tx.send(TrayAction::NextWallpaper);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: lang.tray_stop().into(),
                enabled: has_wall,
                activate: Box::new(|tray: &mut AuraTray| {
                    let _ = tray.tx.send(TrayAction::StopWallpaper);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: lang.tray_quit().into(),
                activate: Box::new(|tray: &mut AuraTray| {
                    let _ = tray.tx.send(TrayAction::QuitApp);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub struct TrayController {
    #[allow(dead_code)]
    pub rx: Arc<TokioMutex<UnboundedReceiver<TrayAction>>>,
    pub current_name: Arc<Mutex<String>>,
    pub is_paused: Arc<Mutex<bool>>,
    pub has_wallpaper: Arc<Mutex<bool>>,
    pub language: Arc<Mutex<crate::i18n::Language>>,
    pub handle: Arc<Mutex<Option<Handle<AuraTray>>>>,
}

static TRAY_RX: std::sync::OnceLock<Arc<TokioMutex<UnboundedReceiver<TrayAction>>>> = std::sync::OnceLock::new();

pub fn get_tray_rx() -> Option<Arc<TokioMutex<UnboundedReceiver<TrayAction>>>> {
    TRAY_RX.get().cloned()
}

impl TrayController {
    pub fn new(lang: crate::i18n::Language) -> (Self, AuraTray) {
        let (tx, rx) = unbounded_channel();
        let rx = Arc::new(TokioMutex::new(rx));
        let _ = TRAY_RX.set(rx.clone());
        let current_name = Arc::new(Mutex::new(String::new()));
        let is_paused = Arc::new(Mutex::new(false));
        let has_wallpaper = Arc::new(Mutex::new(false));
        let language = Arc::new(Mutex::new(lang));
        let handle = Arc::new(Mutex::new(None));

        let tray = AuraTray {
            tx,
            current_name: current_name.clone(),
            is_paused: is_paused.clone(),
            has_wallpaper: has_wallpaper.clone(),
            language: language.clone(),
        };

        let controller = Self {
            rx,
            current_name,
            is_paused,
            has_wallpaper,
            language,
            handle,
        };

        (controller, tray)
    }

    pub fn set_language(&self, lang: crate::i18n::Language) {
        *lock(&self.language) = lang;
        let handle_opt = lock(&self.handle).clone();
        if let Some(handle) = handle_opt {
            tokio::spawn(async move {
                let _ = handle.update(|_| {}).await;
            });
        }
    }

    pub fn update_state(&self, name: String, paused: bool, has_wall: bool) {
        *lock(&self.current_name) = name;
        *lock(&self.is_paused) = paused;
        *lock(&self.has_wallpaper) = has_wall;

        let handle_opt = lock(&self.handle).clone();
        if let Some(handle) = handle_opt {
            tokio::spawn(async move {
                let _ = handle.update(|_| {}).await;
            });
        }
    }

    pub fn spawn_service(&self, tray: AuraTray) {
        let handle_store = self.handle.clone();
        tokio::spawn(async move {
            let is_sandboxed = std::path::Path::new("/.flatpak-info").exists()
                || std::env::var_os("FLATPAK_ID").is_some()
                || std::env::var_os("SNAP").is_some();

            // On boot, desktop panels (e.g. cosmic-panel) and StatusNotifierWatcher may take a few
            // seconds to come online on D-Bus. We use assume_sni_available(true) so that ksni queues
            // registration instead of failing immediately, and retry up to 15 times with backoff.
            for attempt in 1..=15 {
                let builder = tray.clone()
                    .disable_dbus_name(is_sandboxed)
                    .assume_sni_available(true);
                match builder.spawn().await {
                    Ok(handle) => {
                        *lock(&handle_store) = Some(handle);
                        return;
                    }
                    Err(e) => {
                        if attempt == 15 {
                            eprintln!("[Aura Tray] Error al registrar StatusNotifierItem tras 15 intentos: {:?}", e);
                        } else {
                            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                        }
                    }
                }
            }
        });
    }
}
