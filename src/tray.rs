use ksni::menu::StandardItem;
use ksni::{Handle, MenuItem, ToolTip, Tray, TrayMethods};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    ShowApp,
    TogglePause,
    NextWallpaper,
    StopWallpaper,
    QuitApp,
}

pub struct AuraTray {
    pub tx: Sender<TrayAction>,
    pub current_name: Arc<Mutex<String>>,
    pub is_paused: Arc<Mutex<bool>>,
    pub has_wallpaper: Arc<Mutex<bool>>,
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
        let name = self.current_name.lock().unwrap().clone();
        let desc = if name.is_empty() {
            "Sin fondo en reproducción".to_string()
        } else {
            format!("Reproduciendo: {}", name)
        };
        ToolTip {
            title: "Aura • Live Wallpaper".into(),
            description: desc,
            icon_name: "io.github.antwny.aura".into(),
            icon_pixmap: Vec::new(),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayAction::ShowApp);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let is_paused = *self.is_paused.lock().unwrap();
        let has_wall = *self.has_wallpaper.lock().unwrap();

        vec![
            StandardItem {
                label: "Abrir Aura".into(),
                activate: Box::new(|tray: &mut AuraTray| {
                    let _ = tray.tx.send(TrayAction::ShowApp);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: if is_paused {
                    "Reanudar Fondo".into()
                } else {
                    "Pausar Fondo".into()
                },
                enabled: has_wall,
                activate: Box::new(|tray: &mut AuraTray| {
                    let _ = tray.tx.send(TrayAction::TogglePause);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Siguiente Fondo".into(),
                enabled: has_wall,
                activate: Box::new(|tray: &mut AuraTray| {
                    let _ = tray.tx.send(TrayAction::NextWallpaper);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Detener Fondo".into(),
                enabled: has_wall,
                activate: Box::new(|tray: &mut AuraTray| {
                    let _ = tray.tx.send(TrayAction::StopWallpaper);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Salir de Aura".into(),
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
    pub rx: Receiver<TrayAction>,
    pub current_name: Arc<Mutex<String>>,
    pub is_paused: Arc<Mutex<bool>>,
    pub has_wallpaper: Arc<Mutex<bool>>,
    pub handle: Arc<Mutex<Option<Handle<AuraTray>>>>,
}

impl TrayController {
    pub fn new() -> (Self, AuraTray) {
        let (tx, rx) = channel();
        let current_name = Arc::new(Mutex::new(String::new()));
        let is_paused = Arc::new(Mutex::new(false));
        let has_wallpaper = Arc::new(Mutex::new(false));
        let handle = Arc::new(Mutex::new(None));

        let tray = AuraTray {
            tx,
            current_name: current_name.clone(),
            is_paused: is_paused.clone(),
            has_wallpaper: has_wallpaper.clone(),
        };

        let controller = Self {
            rx,
            current_name,
            is_paused,
            has_wallpaper,
            handle,
        };

        (controller, tray)
    }

    pub fn update_state(&self, name: String, paused: bool, has_wall: bool) {
        *self.current_name.lock().unwrap() = name;
        *self.is_paused.lock().unwrap() = paused;
        *self.has_wallpaper.lock().unwrap() = has_wall;

        let handle_opt = self.handle.lock().unwrap().clone();
        if let Some(handle) = handle_opt {
            tokio::spawn(async move {
                let _ = handle.update(|_| {}).await;
            });
        }
    }

    pub fn spawn_service(&self, tray: AuraTray) {
        let handle_store = self.handle.clone();
        tokio::spawn(async move {
            match tray.spawn().await {
                Ok(handle) => {
                    *handle_store.lock().unwrap() = Some(handle);
                }
                Err(e) => {
                    eprintln!("[Aura Tray] Error al registrar StatusNotifierItem: {:?}", e);
                }
            }
        });
    }
}
