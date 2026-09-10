use crate::config::Config;
use crate::engine::{detect_outputs, MonitorOutput, WallpaperEngine};
use crate::scanner::{scan_directories, thumbs::generate_thumbnail, VideoItem};
use crate::theme::apply_cosmic_theme;
use crate::online::{
    download_to_file, fetch_bing_archive_page, fetch_bing_wallpapers,
    fetch_wallhaven_wallpapers, OnlineSource, OnlineWallpaperItem,
};

use cosmic::app::Core;
use cosmic::iced::{Length, Subscription, Task};
use cosmic::widget::{self, nav_bar};
use cosmic::Element;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Library,
    Explore,
    Monitors,
    Settings,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryFilter {
    #[default]
    All,
    Live,
    Static,
    Downloaded,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    SelectPage(nav_bar::Id),
    SelectLibraryFilter(LibraryFilter),
    SetLanguage(crate::i18n::Language),
    SelectOutput(String),
    SelectHwdec(String),
    ToggleRotation(bool),
    SelectInterval(u64),
    SelectRotationOrder(String),
    TogglePauseOnBattery(bool),
    ChangeVolume(u8),
    ApplyWallpaper { video_path: PathBuf, output: String },
    StopWallpaper(Option<String>),
    TogglePause,
    SelectScaling { output: String, scaling: String },
    ToggleAutostart(bool),
    ToggleAutoTheme(bool),
    ToggleAutoDark(bool),
    ToggleMute(bool),
    ToggleSmartPause(bool),
    ToggleKeepRunningOnClose(bool),
    RemoveFolder(String),
    RefreshLibrary,
    ThumbnailGenerated { video_path: PathBuf, thumb_path: PathBuf },
    ThumbnailReadyAndApply { video_path: PathBuf, thumb_path: PathBuf, output: String },
    SearchChanged(String),
    RotationTick,
    HotplugTick,
    SmartPauseTick,
    BatteryTick,
    TickSecond,
    GameModeChanged(bool),
    BatteryStateChanged(Option<bool>),
    OutputsUpdated(Vec<MonitorOutput>),
    PickVideoFile,
    PickFolder,
    FileSelected(Option<PathBuf>),
    FolderSelected(Option<PathBuf>),
    FilesDropped(Vec<PathBuf>),
    DismissStatus,
    OpenGitHub,
    OpenYouTube,
    OpenPayPal,
    TrayAction(crate::tray::TrayAction),
    ShowMainWindow,
    WindowCloseRequested(cosmic::iced::window::Id),
    WindowClosed(cosmic::iced::window::Id),
    NextWallpaper,
    PrevWallpaper,
    QuitApp,
    SelectExploreSource(OnlineSource),
    FetchOnlineWallpapers(OnlineSource),
    OnlineWallpapersFetched(Result<(OnlineSource, Vec<OnlineWallpaperItem>), String>),
    LoadMoreOnlineWallpapers,
    OnlineMoreWallpapersFetched(Result<(OnlineSource, Vec<OnlineWallpaperItem>), String>),
    SelectWallhavenCategory(String),
    SelectWallhavenSorting(String),
    SelectWallhavenResolution(String),
    WallhavenSearchChanged(String),
    SubmitWallhavenSearch,
    DownloadOnlineWallpaper { item: OnlineWallpaperItem, auto_apply: bool },
    OnlineWallpaperDownloaded { id: String, path: PathBuf, auto_apply: bool },
    OnlineWallpaperDownloadFailed { id: String, error: String },
    OnlineThumbLoaded { id: String, path: PathBuf },
    ApplyDownloadedOnlineWallpaper(PathBuf),
    RemoveWallpaperFromLibrary(PathBuf),
    DeleteDownloadedWallpaper(PathBuf),
    DeleteWallpaper(PathBuf),
    CloseToast(cosmic::widget::ToastId),
    OpenWallpapersFolder,
    ShowInFileManager(PathBuf),
    CheckForUpdates { user_initiated: bool },
    UpdateCheckResult { result: Result<(String, String, Option<String>), String>, user_initiated: bool },
    PerformGuiUpdate { download_url: String, version: String },
    GuiUpdateResult(Result<String, String>),
    RestartApp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Available { latest_tag: String, notes: String, download_url: Option<String> },
    Downloading,
    UpdatedSuccess { new_version: String },
    Error(String),
}

fn handle_window_events(
    event: cosmic::iced::Event,
    _status: cosmic::iced::event::Status,
    window: cosmic::iced::window::Id,
) -> Option<Message> {
    match event {
        cosmic::iced::Event::Window(cosmic::iced::window::Event::FileDropped(paths)) => {
            Some(Message::FilesDropped(paths))
        }
        cosmic::iced::Event::Window(cosmic::iced::window::Event::CloseRequested) => {
            Some(Message::WindowCloseRequested(window))
        }
        cosmic::iced::Event::Window(cosmic::iced::window::Event::Closed) => {
            Some(Message::WindowClosed(window))
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Default)]
pub struct AuraFlags {
    pub hidden: bool,
    pub action: Option<String>,
    pub args: Vec<String>,
}

impl cosmic::app::CosmicFlags for AuraFlags {
    type SubCommand = String;
    type Args = Vec<String>;

    fn action(&self) -> Option<&Self::SubCommand> {
        self.action.as_ref()
    }

    fn args(&self) -> Vec<&str> {
        self.args.iter().map(|s| s.as_str()).collect()
    }
}

pub struct AuraApp {
    pub(crate) core: Core,
    pub(crate) nav: nav_bar::Model,
    pub(crate) active_page: Page,
    pub(crate) config: Config,
    pub(crate) language: crate::i18n::Language,
    pub(crate) engine: WallpaperEngine,
    pub(crate) outputs: Vec<MonitorOutput>,
    pub(crate) selected_output: String,
    pub(crate) videos: Vec<VideoItem>,
    pub(crate) search_query: String,
    pub(crate) autostart_active: bool,
    pub(crate) status_message: Option<String>,
    pub(crate) status_timer: u8,
    pub(crate) toasts: cosmic::widget::Toasts<Message>,
    pub(crate) is_paused: bool,
    pub(crate) is_paused_by_smart: bool,
    pub(crate) is_paused_by_battery: bool,
    pub(crate) tray_controller: crate::tray::TrayController,
    pub(crate) is_window_open: bool,
    pub(crate) explore_source: OnlineSource,
    pub(crate) bing_wallpapers: Vec<OnlineWallpaperItem>,
    pub(crate) wallhaven_wallpapers: Vec<OnlineWallpaperItem>,
    pub(crate) explore_loading: bool,
    pub(crate) explore_loading_more: bool,
    pub(crate) explore_error: Option<String>,
    pub(crate) downloading_online_ids: std::collections::HashSet<String>,
    pub(crate) online_thumbs: std::collections::HashMap<String, PathBuf>,
    pub(crate) http_client: reqwest::Client,
    pub(crate) bing_page: u32,
    pub(crate) wallhaven_page: u32,
    pub(crate) wallhaven_category: String,
    pub(crate) wallhaven_sorting: String,
    pub(crate) wallhaven_resolution: String,
    pub(crate) wallhaven_search: String,
    pub(crate) library_filter: LibraryFilter,
    pub(crate) update_status: UpdateStatus,
}

impl AuraApp {
    pub(crate) fn notify(&mut self, text: impl Into<String>) -> Task<cosmic::Action<Message>> {
        let task = self.toasts.push(
            cosmic::widget::Toast::new(text).duration(cosmic::widget::toaster::Duration::Short)
        );
        task.map(cosmic::Action::App)
    }

    pub(crate) fn notify_with_action(
        &mut self,
        text: impl Into<String>,
        action_label: impl Into<String>,
        action_msg: Message,
    ) -> Task<cosmic::Action<Message>> {
        let toast = cosmic::widget::Toast::new(text)
            .duration(cosmic::widget::toaster::Duration::Long)
            .action(action_label.into(), move |_id| action_msg.clone());
        let task = self.toasts.push(toast);
        task.map(cosmic::Action::App)
    }

    pub(crate) fn set_status(&mut self, text: impl Into<String>) -> Task<cosmic::Action<Message>> {
        let msg = text.into();
        self.status_message = Some(msg.clone());
        self.notify(msg)
    }
}

impl cosmic::Application for AuraApp {
    type Executor = cosmic::executor::Default;
    type Flags = AuraFlags;
    type Message = Message;
    const APP_ID: &'static str = "io.github.antwny.aura";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let config = Config::load();
        let _ = config.save();
        let language = crate::i18n::Language::from_code(&config.language);
        let nav = Self::build_nav(language, Page::Library);

        let outputs = detect_outputs();
        let selected_output = if !config.output.is_empty() {
            config.output.clone()
        } else {
            outputs.first().map(|o| o.name.clone()).unwrap_or_else(|| "*".into())
        };

        let autostart_active = config.autostart || WallpaperEngine::is_autostart_enabled();
        if autostart_active && !WallpaperEngine::is_autostart_enabled() {
            let _ = WallpaperEngine::write_autostart();
        }

        let videos = scan_directories(&config.dirs, &config.custom_videos);

        // Spawn async background thumbnail generation
        let mut tasks = Vec::new();
        if flags.hidden {
            core.set_main_window_id(None);
        }

        for item in &videos {
            if item.thumb_path.is_none() {
                let v_path = item.path.clone();
                tasks.push(Task::perform(
                    async move {
                        let res = generate_thumbnail(&v_path).await;
                        (v_path, res)
                    },
                    |(video_path, res)| {
                        if let Some(thumb_path) = res {
                            cosmic::Action::App(Message::ThumbnailGenerated { video_path, thumb_path })
                        } else {
                            cosmic::Action::None
                        }
                    },
                ));
            }
        }

        let (tray_controller, tray) = crate::tray::TrayController::new(language);
        let current_title = if let Some(curr) = &config.current {
            std::path::Path::new(curr)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };
        tray_controller.update_state(current_title, false, !config.wallpapers.is_empty());
        tray_controller.spawn_service(tray);

        let mut engine = WallpaperEngine::new();
        let has_initial_action = flags.action.is_some();
        if !has_initial_action {
            for (out, path) in &config.wallpapers {
                let sc = config.scaling.get(out).cloned().unwrap_or_else(|| "fit".into());
                if std::path::Path::new(path).exists() {
                    let _ = engine.set_wallpaper(out, path, &sc, config.mute, config.volume, &config.hwdec);
                }
            }
        }

        if let Some(action) = &flags.action {
            match action.as_str() {
                "apply" => {
                    if let Some(path) = flags.args.first() {
                        let path_buf = PathBuf::from(path);
                        tasks.push(Task::done(cosmic::Action::App(Message::ApplyWallpaper {
                            video_path: path_buf,
                            output: selected_output.clone(),
                        })));
                    }
                }
                "next" => tasks.push(Task::done(cosmic::Action::App(Message::NextWallpaper))),
                "prev" => tasks.push(Task::done(cosmic::Action::App(Message::PrevWallpaper))),
                "stop" => {
                    engine.stop_all();
                    std::process::exit(0);
                }
                "toggle-pause" | "pause" => {
                    tasks.push(Task::done(cosmic::Action::App(Message::TogglePause)));
                }
                _ => {}
            }
        }

        let is_window_open = !flags.hidden;

        let app = Self {
            core,
            nav,
            active_page: Page::Library,
            config,
            language,
            engine,
            outputs,
            selected_output,
            videos,
            search_query: String::new(),
            autostart_active,
            status_message: None,
            status_timer: 0,
            toasts: cosmic::widget::Toasts::new(Message::CloseToast),
            is_paused: false,
            is_paused_by_smart: false,
            is_paused_by_battery: false,
            tray_controller,
            is_window_open,
            explore_source: OnlineSource::Bing,
            bing_wallpapers: Vec::new(),
            wallhaven_wallpapers: Vec::new(),
            explore_loading: false,
            explore_loading_more: false,
            explore_error: None,
            downloading_online_ids: std::collections::HashSet::new(),
            online_thumbs: std::collections::HashMap::new(),
            http_client: reqwest::Client::new(),
            bing_page: 1,
            wallhaven_page: 1,
            wallhaven_category: "110".into(),
            wallhaven_sorting: "toplist".into(),
            wallhaven_resolution: "all".into(),
            wallhaven_search: String::new(),
            library_filter: LibraryFilter::All,
            update_status: UpdateStatus::Idle,
        };

        if is_window_open && !crate::online::updater::is_flatpak() {
            tasks.push(Task::done(cosmic::Action::App(Message::CheckForUpdates { user_initiated: false })));
        }

        (app, Task::batch(tasks))
    }

    fn on_close_requested(&self, id: cosmic::iced::window::Id) -> Option<Self::Message> {
        Some(Message::WindowCloseRequested(id))
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        self.nav.activate(id);
        if let Some(&page) = self.nav.active_data::<Page>() {
            self.active_page = page;
            if page == Page::Explore {
                let need_fetch = match self.explore_source {
                    OnlineSource::Bing => self.bing_wallpapers.is_empty(),
                    OnlineSource::Wallhaven => self.wallhaven_wallpapers.is_empty(),
                };
                if need_fetch && !self.explore_loading {
                    return Task::done(cosmic::Action::App(Message::FetchOnlineWallpapers(self.explore_source)));
                }
            }
        }
        Task::none()
    }

    fn dbus_activation(&mut self, msg: cosmic::dbus_activation::Message) -> Task<cosmic::Action<Self::Message>> {
        match msg.msg {
            cosmic::dbus_activation::Details::Activate => {
                Task::done(cosmic::Action::App(Message::ShowMainWindow))
            }
            cosmic::dbus_activation::Details::ActivateAction { action, args } => {
                match action.as_str() {
                    "apply" => {
                        if let Some(path) = args.first() {
                            let path_buf = PathBuf::from(path);
                            Task::done(cosmic::Action::App(Message::ApplyWallpaper {
                                video_path: path_buf,
                                output: self.selected_output.clone(),
                            }))
                        } else {
                            Task::none()
                        }
                    }
                    "next" => Task::done(cosmic::Action::App(Message::NextWallpaper)),
                    "prev" => Task::done(cosmic::Action::App(Message::PrevWallpaper)),
                    "stop" => Task::done(cosmic::Action::App(Message::StopWallpaper(None))),
                    "toggle-pause" | "pause" => Task::done(cosmic::Action::App(Message::TogglePause)),
                    _ => Task::none(),
                }
            }
            cosmic::dbus_activation::Details::Open { url } => {
                if let Some(first) = url.first() {
                    if let Ok(path) = first.to_file_path() {
                        return Task::done(cosmic::Action::App(Message::ApplyWallpaper {
                            video_path: path,
                            output: self.selected_output.clone(),
                        }));
                    }
                }
                Task::none()
            }
        }
    }

    fn view_window(&self, _id: cosmic::iced::window::Id) -> Element<'_, Self::Message> {
        self.view()
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        vec![
            Element::from(widget::text::title3("Aura")),
            Element::from(widget::text::body(self.language.header_wallpapers_count(self.videos.len()))),
        ]
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        vec![
            Element::from(
                widget::button::suggested(self.language.header_add_video())
                    .leading_icon(widget::icon::from_name("list-add-symbolic"))
                    .on_press(Message::PickVideoFile)
            ),
            Element::from(
                widget::button::standard(self.language.header_add_folder())
                    .leading_icon(widget::icon::from_name("folder-symbolic"))
                    .on_press(Message::PickFolder)
            ),
        ]
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let mut subs = Vec::new();

        subs.push(cosmic::iced::event::listen_with(handle_window_events));

        if self.status_timer > 0 {
            subs.push(
                cosmic::iced::time::every(Duration::from_secs(1))
                    .map(|_| Message::TickSecond)
            );
        }

        fn tray_stream() -> impl cosmic::iced::futures::Stream<Item = Message> {
            cosmic::iced::stream::channel(10, |mut output: cosmic::iced::futures::channel::mpsc::Sender<Message>| {
                async move {
                    use cosmic::iced::futures::SinkExt;
                    if let Some(rx) = crate::tray::get_tray_rx() {
                        let mut rx = rx.lock().await;
                        while let Some(action) = rx.recv().await {
                            let _ = output.send(Message::TrayAction(action)).await;
                        }
                    }
                }
            })
        }
        subs.push(Subscription::run(tray_stream));

        if self.config.rotation && self.config.interval > 0 {
            subs.push(
                cosmic::iced::time::every(Duration::from_secs(self.config.interval * 60))
                    .map(|_| Message::RotationTick)
            );
        }

        subs.push(
            cosmic::iced::time::every(Duration::from_secs(5))
                .map(|_| Message::HotplugTick)
        );

        if self.config.smart_pause && !self.config.wallpapers.is_empty() {
            subs.push(
                cosmic::iced::time::every(Duration::from_secs(3))
                    .map(|_| Message::SmartPauseTick)
            );
        }

        if self.config.pause_on_battery && !self.config.wallpapers.is_empty() {
            subs.push(
                cosmic::iced::time::every(Duration::from_secs(4))
                    .map(|_| Message::BatteryTick)
            );
        }

        Subscription::batch(subs)
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::SelectPage(id) => {
                self.nav.activate(id);
                if let Some(&page) = self.nav.active_data::<Page>() {
                    self.active_page = page;
                    if page == Page::Explore {
                        let need_fetch = match self.explore_source {
                            OnlineSource::Bing => self.bing_wallpapers.is_empty(),
                            OnlineSource::Wallhaven => self.wallhaven_wallpapers.is_empty(),
                        };
                        if need_fetch && !self.explore_loading {
                            return Task::done(cosmic::Action::App(Message::FetchOnlineWallpapers(self.explore_source)));
                        }
                    }
                }
            }

            Message::SelectOutput(out) => {
                self.selected_output = out.clone();
                self.config.output = out.clone();
                let _ = self.config.save();
                return self.set_status(format!("Pantalla seleccionada: {}", out));
            }

            Message::SelectHwdec(hwdec) => {
                self.config.hwdec = hwdec.clone();
                let _ = self.config.save();
                for (output, path) in self.config.wallpapers.clone() {
                    let sc = self.config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());
                    let _ = self.engine.set_wallpaper(&output, &path, &sc, self.config.mute, self.config.volume, &self.config.hwdec);
                }
                return self.set_status(format!("Decodificador GPU: {}", hwdec));
            }

            Message::ToggleRotation(active) => {
                self.config.rotation = active;
                let _ = self.config.save();
                return self.set_status(if active { "Rotación automática activada" } else { "Rotación automática desactivada" });
            }

            Message::SelectInterval(interval) => {
                self.config.interval = interval;
                let _ = self.config.save();
                return self.set_status(format!("Intervalo de rotación: {} min", interval));
            }

            Message::SelectRotationOrder(order) => {
                self.config.order = order.clone();
                let _ = self.config.save();
                return self.set_status(format!("Orden de rotación: {}", if order == "random" { "Aleatorio" } else { "Secuencial" }));
            }

            Message::TogglePauseOnBattery(active) => {
                self.config.pause_on_battery = active;
                let _ = self.config.save();
                return self.set_status(if active { "Ahorro de batería activado" } else { "Ahorro de batería desactivado" });
            }

            Message::ChangeVolume(vol) => {
                self.config.volume = vol;
                let _ = self.config.save();
                if !self.config.mute {
                    for (output, path) in self.config.wallpapers.clone() {
                        let sc = self.config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());
                        let _ = self.engine.set_wallpaper(&output, &path, &sc, false, self.config.volume, &self.config.hwdec);
                    }
                }
            }

            Message::CloseToast(id) => {
                self.toasts.remove(id);
            }

            Message::OpenWallpapersFolder => {
                let dir = crate::online::wallpapers_online_dir();
                let _ = std::fs::create_dir_all(&dir);
                if open::that_detached(&dir).is_ok() {
                    return self.notify(self.language.toast_folder_opened());
                } else {
                    return self.notify(self.language.toast_folder_open_failed());
                }
            }

            Message::ShowInFileManager(path) => {
                let target = if path.is_file() {
                    path.parent().map(|p| p.to_path_buf()).unwrap_or(path)
                } else {
                    path
                };
                if open::that_detached(&target).is_ok() {
                    return self.notify(self.language.toast_folder_opened());
                } else {
                    return self.notify(self.language.toast_folder_open_failed());
                }
            }

            Message::TickSecond => {
                if self.status_timer > 0 {
                    self.status_timer -= 1;
                    if self.status_timer == 0 {
                        self.status_message = None;
                    }
                }
            }

            Message::DismissStatus => {
                self.status_message = None;
                self.status_timer = 0;
            }

            Message::TrayAction(action) => match action {
                crate::tray::TrayAction::ShowApp => {
                    return Task::done(cosmic::Action::App(Message::ShowMainWindow));
                }
                crate::tray::TrayAction::TogglePause => {
                    return Task::done(cosmic::Action::App(Message::TogglePause));
                }
                crate::tray::TrayAction::NextWallpaper => {
                    return Task::done(cosmic::Action::App(Message::NextWallpaper));
                }
                crate::tray::TrayAction::StopWallpaper => {
                    return Task::done(cosmic::Action::App(Message::StopWallpaper(None)));
                }
                crate::tray::TrayAction::QuitApp => {
                    return Task::done(cosmic::Action::App(Message::QuitApp));
                }
            }

            Message::ShowMainWindow => {
                if !self.is_window_open || self.core().main_window_id().is_none() {
                    let mut win_settings = cosmic::iced::window::Settings::default();
                    win_settings.size = cosmic::iced::Size::new(1024.0, 720.0);
                    win_settings.min_size = Some(cosmic::iced::Size::new(840.0, 580.0));
                    win_settings.exit_on_close_request = false;
                    win_settings.decorations = false;
                    win_settings.transparent = true;
                    win_settings.resizable = true;
                    win_settings.resize_border = 8;
                    #[cfg(target_os = "linux")]
                    {
                        win_settings.platform_specific.application_id = Self::APP_ID.to_string();
                    }

                    let (new_id, open_task) = cosmic::iced::window::open(win_settings);
                    self.is_window_open = true;
                    self.core_mut().set_main_window_id(Some(new_id));
                    return Task::batch([open_task.discard(), cosmic::iced::window::gain_focus(new_id)]);
                } else if let Some(id) = self.core().main_window_id() {
                    return Task::batch([
                        cosmic::iced::window::minimize(id, false),
                        cosmic::iced::window::gain_focus(id),
                    ]);
                }
            }

            Message::WindowCloseRequested(id) => {
                if self.config.keep_running_on_close {
                    self.is_window_open = false;
                    self.core_mut().set_main_window_id(None);
                    return cosmic::iced::window::close(id);
                } else {
                    self.engine.stop_all();
                    std::process::exit(0);
                }
            }

            Message::WindowClosed(id) => {
                self.is_window_open = false;
                if self.core().main_window_id() == Some(id) {
                    self.core_mut().set_main_window_id(None);
                }
            }

            Message::NextWallpaper => {
                if !self.videos.is_empty() {
                    let next_idx = (self.config.seq_index + 1) % self.videos.len();
                    self.config.seq_index = next_idx;
                    let _ = self.config.save();
                    let video = &self.videos[next_idx];
                    let output = self.selected_output.clone();
                    return Task::done(cosmic::Action::App(Message::ApplyWallpaper {
                        video_path: video.path.clone(),
                        output,
                    }));
                }
            }

            Message::PrevWallpaper => {
                if !self.videos.is_empty() {
                    let prev_idx = if self.config.seq_index == 0 {
                        self.videos.len() - 1
                    } else {
                        self.config.seq_index - 1
                    };
                    self.config.seq_index = prev_idx;
                    let _ = self.config.save();
                    let video = &self.videos[prev_idx];
                    let output = self.selected_output.clone();
                    return Task::done(cosmic::Action::App(Message::ApplyWallpaper {
                        video_path: video.path.clone(),
                        output,
                    }));
                }
            }

            Message::SetLanguage(lang) => {
                self.language = lang;
                self.config.language = lang.code().to_string();
                let _ = self.config.save();
                self.nav = Self::build_nav(self.language, self.active_page);
                self.tray_controller.set_language(lang);
                return self.set_status(lang.status_lang_changed());
            }

            Message::QuitApp => {
                self.engine.stop_all();
                std::process::exit(0);
            }

            Message::OpenGitHub => {
                let _ = open::that_detached("https://github.com/antwny/aura");
            }

            Message::OpenYouTube => {
                let _ = open::that_detached("https://www.youtube.com/@antwny");
            }

            Message::OpenPayPal => {
                let _ = open::that_detached("https://www.paypal.com/donate/?business=antwnyab@gmail.com&no_recurring=0&currency_code=USD");
            }

            Message::CheckForUpdates { user_initiated } => {
                self.update_status = UpdateStatus::Checking;
                let client = self.http_client.clone();
                return Task::perform(
                    async move {
                        crate::online::updater::check_latest_release(&client).await
                    },
                    move |res| cosmic::Action::App(Message::UpdateCheckResult { result: res, user_initiated }),
                );
            }

            Message::UpdateCheckResult { result, user_initiated } => {
                match result {
                    Ok((latest_tag, notes, download_url)) => {
                        let clean_latest = latest_tag.trim_start_matches('v');
                        let current_version = env!("CARGO_PKG_VERSION");
                        if crate::online::updater::is_newer_version(clean_latest, current_version) {
                            self.update_status = UpdateStatus::Available {
                                latest_tag: latest_tag.clone(),
                                notes,
                                download_url: download_url.clone(),
                            };
                            let toast_text = format!("🚀 {} {}", self.language.about_update_available(), latest_tag);
                            if let Some(url) = download_url {
                                let action_lbl = self.language.toast_update_available_action();
                                return self.notify_with_action(
                                    toast_text,
                                    action_lbl,
                                    Message::PerformGuiUpdate { download_url: url, version: latest_tag },
                                );
                            } else {
                                return self.notify_with_action(
                                    toast_text,
                                    self.language.about_github_btn(),
                                    Message::OpenGitHub,
                                );
                            }
                        } else {
                            self.update_status = UpdateStatus::UpToDate;
                            if user_initiated {
                                return self.notify(self.language.about_up_to_date());
                            }
                        }
                    }
                    Err(e) => {
                        self.update_status = UpdateStatus::Error(e.clone());
                        if user_initiated {
                            return self.notify(format!("Error: {}", e));
                        }
                    }
                }
            }

            Message::PerformGuiUpdate { download_url, version } => {
                self.update_status = UpdateStatus::Downloading;
                let client = self.http_client.clone();
                let notify_task = self.notify(self.language.about_updating());
                let update_task = Task::perform(
                    async move {
                        match crate::online::updater::perform_update(&client, &download_url).await {
                            Ok(_) => Ok(version),
                            Err(e) => Err(e),
                        }
                    },
                    |res| cosmic::Action::App(Message::GuiUpdateResult(res)),
                );
                return Task::batch(vec![notify_task, update_task]);
            }

            Message::GuiUpdateResult(res) => {
                match res {
                    Ok(ver) => {
                        self.update_status = UpdateStatus::UpdatedSuccess { new_version: ver.clone() };
                        return self.notify_with_action(
                            format!("🎉 ¡Aura actualizada a {}!", ver),
                            self.language.toast_restart_action(),
                            Message::RestartApp,
                        );
                    }
                    Err(e) => {
                        self.update_status = UpdateStatus::Error(e.clone());
                        return self.notify(format!("Error al actualizar: {}", e));
                    }
                }
            }

            Message::RestartApp => {
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).spawn();
                }
                std::process::exit(0);
            }

            Message::ApplyWallpaper { video_path, output } => {
                let scaling = self.config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());
                let mute = self.config.mute;
                let volume = self.config.volume;
                let hwdec = self.config.hwdec.clone();

                let path_str = video_path.to_string_lossy().to_string();
                if let Ok(_) = self.engine.set_wallpaper(&output, &path_str, &scaling, mute, volume, &hwdec) {
                    self.config.wallpapers.insert(output.clone(), path_str.clone());
                    self.config.current = Some(path_str.clone());
                    let _ = self.config.save();
                    self.is_paused = false;

                    let file_name = video_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Video".into());
                    self.tray_controller.update_state(file_name.clone(), false, true);

                    // COSMIC dynamic accent theme
                    if self.config.auto_theme {
                        if let Some(thumb) = self.videos.iter().find(|v| v.path == video_path).and_then(|v| v.thumb_path.as_ref()) {
                            apply_cosmic_theme(thumb, self.config.auto_dark);
                        } else if let Some(ext) = video_path.extension().and_then(|e| e.to_str()) {
                            if crate::scanner::is_supported_wallpaper_ext(ext) {
                                apply_cosmic_theme(&video_path, self.config.auto_dark);
                            }
                        }
                    }

                    return self.set_status(self.language.status_applied(&output, &file_name));
                }
            }

            Message::TogglePause => {
                let now_paused = self.engine.toggle_pause();
                self.is_paused = now_paused;

                let current_title = self.config.current.as_ref()
                    .and_then(|c| std::path::Path::new(c).file_stem().map(|s| s.to_string_lossy().to_string()))
                    .unwrap_or_default();
                self.tray_controller.update_state(current_title, now_paused, !self.config.wallpapers.is_empty());

                let msg = if now_paused { self.language.status_paused() } else { self.language.status_resumed() };
                return self.set_status(msg);
            }

            Message::StopWallpaper(output) => {
                let msg = if let Some(out) = &output {
                    self.engine.stop_output(out);
                    self.config.wallpapers.remove(out);
                    self.language.status_stopped_output(out)
                } else {
                    self.engine.stop_all();
                    self.config.wallpapers.clear();
                    self.config.current = None;
                    self.language.status_stopped_all().into()
                };
                self.is_paused = false;
                let _ = self.config.save();

                let current_title = self.config.current.as_ref()
                    .and_then(|c| std::path::Path::new(c).file_stem().map(|s| s.to_string_lossy().to_string()))
                    .unwrap_or_default();
                self.tray_controller.update_state(current_title, false, !self.config.wallpapers.is_empty());

                return self.set_status(msg);
            }

            Message::SelectScaling { output, scaling } => {
                self.config.scaling.insert(output.clone(), scaling.clone());
                let _ = self.config.save();
                if let Some(path) = self.config.wallpapers.get(&output).cloned() {
                    let _ = self.engine.set_wallpaper(&output, &path, &scaling, self.config.mute, self.config.volume, &self.config.hwdec);
                    self.status_message = Some(format!("Modo de escala para {} cambiado a '{}'", output, scaling));
                    self.status_timer = 5;
                }
            }

            Message::ToggleAutostart(active) => {
                self.autostart_active = active;
                self.config.autostart = active;
                if active {
                    let _ = WallpaperEngine::write_autostart();
                    self.status_message = Some(match self.language {
                        crate::i18n::Language::Es => "Inicio automático activado".into(),
                        crate::i18n::Language::En => "Autostart enabled".into(),
                    });
                } else {
                    let _ = WallpaperEngine::remove_autostart();
                    self.status_message = Some(match self.language {
                        crate::i18n::Language::Es => "Inicio automático desactivado".into(),
                        crate::i18n::Language::En => "Autostart disabled".into(),
                    });
                }
                let _ = self.config.save();
                self.status_timer = 5;
            }

            Message::ToggleAutoTheme(active) => {
                self.config.auto_theme = active;
                let _ = self.config.save();
            }

            Message::ToggleAutoDark(active) => {
                self.config.auto_dark = active;
                let _ = self.config.save();
            }

            Message::ToggleSmartPause(active) => {
                self.config.smart_pause = active;
                let _ = self.config.save();
            }

            Message::ToggleKeepRunningOnClose(active) => {
                self.config.keep_running_on_close = active;
                let _ = self.config.save();
            }

            Message::ToggleMute(mute) => {
                self.config.mute = mute;
                let _ = self.config.save();
                for (output, path) in self.config.wallpapers.clone() {
                    let sc = self.config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());
                    let _ = self.engine.set_wallpaper(&output, &path, &sc, mute, self.config.volume, &self.config.hwdec);
                }
                self.status_message = Some(if mute { self.language.status_muted().into() } else { self.language.status_unmuted().into() });
                self.status_timer = 5;
            }

            Message::RemoveFolder(folder) => {
                self.config.dirs.retain(|d| d != &folder);
                let _ = self.config.save();
                self.videos = scan_directories(&self.config.dirs, &self.config.custom_videos);
                self.status_message = Some(self.language.status_folder_removed(&folder));
                self.status_timer = 5;
            }

            Message::RefreshLibrary => {
                self.videos = scan_directories(&self.config.dirs, &self.config.custom_videos);
                self.status_message = Some(self.language.status_library_refreshed().into());
                self.status_timer = 5;
            }

            Message::ThumbnailGenerated { video_path, thumb_path } => {
                if let Some(item) = self.videos.iter_mut().find(|v| v.path == video_path) {
                    item.thumb_path = Some(thumb_path);
                }
            }

            Message::ThumbnailReadyAndApply { video_path, thumb_path, output } => {
                if let Some(item) = self.videos.iter_mut().find(|v| v.path == video_path) {
                    item.thumb_path = Some(thumb_path);
                }
                return Task::done(cosmic::Action::App(Message::ApplyWallpaper {
                    video_path,
                    output,
                }));
            }

            Message::SearchChanged(q) => {
                self.search_query = q;
            }

            Message::PickVideoFile => {
                let title = self.language.dialog_pick_video();
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title(title)
                            .add_filter("Supported Media", &["mp4", "webm", "mkv", "avi", "mov", "jpg", "jpeg", "png", "webp"])
                            .add_filter("Videos", &["mp4", "webm", "mkv", "avi", "mov"])
                            .add_filter("Images", &["jpg", "jpeg", "png", "webp"])
                            .pick_file()
                            .await;
                        file.map(|f| f.path().to_path_buf())
                    },
                    |res| cosmic::Action::App(Message::FileSelected(res)),
                );
            }

            Message::FileSelected(Some(path)) => {
                let path_str = path.to_string_lossy().to_string();
                if !self.config.custom_videos.contains(&path_str) {
                    self.config.custom_videos.push(path_str.clone());
                    let _ = self.config.save();
                }
                self.videos = scan_directories(&self.config.dirs, &self.config.custom_videos);
                let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                self.status_message = Some(self.language.status_video_added(&file_name));
                self.status_timer = 5;

                let output = self.selected_output.clone();
                let is_image = path.extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| crate::scanner::IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false);

                if is_image {
                    return Task::done(cosmic::Action::App(Message::ApplyWallpaper { video_path: path, output }));
                }

                let v_path = path.clone();
                return Task::perform(
                    async move {
                        let thumb = generate_thumbnail(&v_path).await;
                        (v_path, thumb)
                    },
                    move |(v_path, thumb)| {
                        if let Some(thumb_path) = thumb {
                            cosmic::Action::App(Message::ThumbnailReadyAndApply { video_path: v_path, thumb_path, output })
                        } else {
                            cosmic::Action::App(Message::ApplyWallpaper { video_path: v_path, output })
                        }
                    },
                );
            }
            Message::FileSelected(None) => {}

            Message::PickFolder => {
                let title = self.language.dialog_pick_folder();
                return Task::perform(
                    async move {
                        let folder = rfd::AsyncFileDialog::new()
                            .set_title(title)
                            .pick_folder()
                            .await;
                        folder.map(|f| f.path().to_path_buf())
                    },
                    |res| cosmic::Action::App(Message::FolderSelected(res)),
                );
            }

            Message::FolderSelected(Some(path)) => {
                let folder_str = path.to_string_lossy().to_string();
                if !self.config.dirs.contains(&folder_str) {
                    self.config.dirs.push(folder_str.clone());
                    let _ = self.config.save();
                }
                self.videos = scan_directories(&self.config.dirs, &self.config.custom_videos);
                self.status_message = Some(self.language.status_folder_added(&folder_str));
                self.status_timer = 5;

                let mut tasks = Vec::new();
                for item in &self.videos {
                    if item.thumb_path.is_none() {
                        let vp = item.path.clone();
                        tasks.push(Task::perform(
                            async move { (vp.clone(), generate_thumbnail(&vp).await) },
                            |(vp, t)| {
                                if let Some(thumb_path) = t {
                                    cosmic::Action::App(Message::ThumbnailGenerated { video_path: vp, thumb_path })
                                } else {
                                    cosmic::Action::None
                                }
                            },
                        ));
                    }
                }
                return Task::batch(tasks);
            }
            Message::FolderSelected(None) => {}

            Message::FilesDropped(paths) => {
                let mut added_any = false;
                let mut apply_last: Option<PathBuf> = None;

                for p in paths {
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        if crate::scanner::is_supported_wallpaper_ext(ext) {
                            let p_str = p.to_string_lossy().to_string();
                            if !self.config.custom_videos.contains(&p_str) {
                                self.config.custom_videos.push(p_str);
                                added_any = true;
                                apply_last = Some(p.clone());
                            }
                        }
                    }
                }

                if added_any {
                    let _ = self.config.save();
                    self.videos = scan_directories(&self.config.dirs, &self.config.custom_videos);
                    self.status_message = Some(self.language.status_videos_dropped().into());
                    self.status_timer = 5;

                    if let Some(media_to_apply) = apply_last {
                        let out = self.selected_output.clone();
                        let is_img = media_to_apply.extension()
                            .and_then(|e| e.to_str())
                            .map(|ext| crate::scanner::IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                            .unwrap_or(false);

                        if is_img {
                            return Task::done(cosmic::Action::App(Message::ApplyWallpaper {
                                video_path: media_to_apply,
                                output: out,
                            }));
                        } else {
                            let vp = media_to_apply.clone();
                            return Task::perform(
                                async move { (vp.clone(), generate_thumbnail(&vp).await) },
                                move |(vp, t)| {
                                    if let Some(thumb_path) = t {
                                        cosmic::Action::App(Message::ThumbnailReadyAndApply { video_path: vp, thumb_path, output: out })
                                    } else {
                                        cosmic::Action::App(Message::ApplyWallpaper { video_path: vp, output: out })
                                    }
                                },
                            );
                        }
                    }
                }
            }

            Message::HotplugTick => {
                let current_outputs = detect_outputs();
                if current_outputs != self.outputs {
                    let new_count = current_outputs.len();
                    self.outputs = current_outputs;
                    self.status_message = Some(self.language.status_topology_updated(new_count));
                    self.status_timer = 5;
                }
            }

            Message::OutputsUpdated(new_outs) => {
                self.outputs = new_outs;
            }

            Message::SmartPauseTick => {
                if self.config.smart_pause && !self.config.wallpapers.is_empty() {
                    return Task::perform(
                        crate::system::check_if_fullscreen_game_active(),
                        |in_game| cosmic::Action::App(Message::GameModeChanged(in_game)),
                    );
                }
            }

            Message::GameModeChanged(in_game) => {
                if in_game && !self.is_paused {
                    self.engine.pause_all();
                    self.is_paused = true;
                    self.is_paused_by_smart = true;
                    self.status_message = Some(self.language.status_game_paused().into());
                    self.status_timer = 5;
                } else if !in_game && self.is_paused && self.is_paused_by_smart {
                    self.engine.resume_all();
                    self.is_paused = false;
                    self.is_paused_by_smart = false;
                    self.status_message = Some(self.language.status_game_resumed().into());
                    self.status_timer = 5;
                }
            }

            Message::BatteryTick => {
                if self.config.pause_on_battery && !self.config.wallpapers.is_empty() {
                    return Task::perform(
                        crate::system::check_on_battery(),
                        |b| cosmic::Action::App(Message::BatteryStateChanged(b)),
                    );
                }
            }

            Message::BatteryStateChanged(Some(on_battery)) => {
                if on_battery && !self.is_paused && self.config.pause_on_battery {
                    self.engine.pause_all();
                    self.is_paused = true;
                    self.is_paused_by_battery = true;
                    self.status_message = Some(self.language.status_battery_paused().into());
                    self.status_timer = 5;
                } else if !on_battery && self.is_paused && self.is_paused_by_battery {
                    self.engine.resume_all();
                    self.is_paused = false;
                    self.is_paused_by_battery = false;
                    self.status_message = Some(self.language.status_battery_resumed().into());
                    self.status_timer = 5;
                }
            }
            Message::BatteryStateChanged(None) => {}

            Message::RotationTick => {
                if !self.videos.is_empty() && self.config.rotation {
                    let next_idx = if self.config.order == "random" {
                        (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as usize) % self.videos.len()
                    } else {
                        (self.config.seq_index + 1) % self.videos.len()
                    };
                    self.config.seq_index = next_idx;
                    let _ = self.config.save();
                    let video = &self.videos[next_idx];
                    let output = self.selected_output.clone();
                    return Task::done(cosmic::Action::App(Message::ApplyWallpaper {
                        video_path: video.path.clone(),
                        output,
                    }));
                }
            }

            Message::SelectLibraryFilter(filter) => {
                self.library_filter = filter;
            }

            Message::SelectExploreSource(source) => {
                self.explore_source = source;
                let need_fetch = match source {
                    OnlineSource::Bing => self.bing_wallpapers.is_empty(),
                    OnlineSource::Wallhaven => self.wallhaven_wallpapers.is_empty(),
                };
                if need_fetch && !self.explore_loading {
                    return Task::done(cosmic::Action::App(Message::FetchOnlineWallpapers(source)));
                }
            }

            Message::FetchOnlineWallpapers(source) => {
                self.explore_loading = true;
                self.explore_error = None;
                let client = self.http_client.clone();
                let cat = self.wallhaven_category.clone();
                let sort = self.wallhaven_sorting.clone();
                let res = self.wallhaven_resolution.clone();
                let search = if self.wallhaven_search.trim().is_empty() {
                    None
                } else {
                    Some(self.wallhaven_search.clone())
                };

                return Task::perform(
                    async move {
                        match source {
                            OnlineSource::Bing => {
                                fetch_bing_wallpapers(&client)
                                    .await
                                    .map(|items| (source, items))
                            }
                            OnlineSource::Wallhaven => {
                                fetch_wallhaven_wallpapers(&client, 1, search.as_deref(), &cat, &sort, &res)
                                    .await
                                    .map(|items| (source, items))
                            }
                        }
                    },
                    |res| cosmic::Action::App(Message::OnlineWallpapersFetched(res)),
                );
            }

            Message::OnlineWallpapersFetched(result) => {
                self.explore_loading = false;
                match result {
                    Ok((source, items)) => {
                        match source {
                            OnlineSource::Bing => self.bing_wallpapers = items.clone(),
                            OnlineSource::Wallhaven => self.wallhaven_wallpapers = items.clone(),
                        }
                        self.explore_error = None;

                        let mut thumb_tasks = Vec::new();
                        let client = self.http_client.clone();
                        for item in items {
                            let thumb_dest = item.local_thumb_path();
                            if thumb_dest.exists() {
                                self.online_thumbs.insert(item.id.clone(), thumb_dest);
                            } else {
                                let c = client.clone();
                                let id = item.id.clone();
                                let url = item.thumb_url.clone();
                                thumb_tasks.push(Task::perform(
                                    async move {
                                        if let Ok(path) = download_to_file(&c, &url, &thumb_dest).await {
                                            Some((id, path))
                                        } else {
                                            None
                                        }
                                    },
                                    |res| {
                                        if let Some((id, path)) = res {
                                            cosmic::Action::App(Message::OnlineThumbLoaded { id, path })
                                        } else {
                                            cosmic::Action::None
                                        }
                                    },
                                ));
                            }
                        }
                        if !thumb_tasks.is_empty() {
                            return Task::batch(thumb_tasks);
                        }
                    }
                    Err(err) => {
                        self.explore_error = Some(err);
                    }
                }
            }

            Message::LoadMoreOnlineWallpapers => {
                if self.explore_loading_more || self.explore_loading {
                    return Task::none();
                }
                self.explore_loading_more = true;
                let client = self.http_client.clone();
                let source = self.explore_source;

                match source {
                    OnlineSource::Bing => {
                        self.bing_page += 1;
                        let page = self.bing_page;
                        return Task::perform(
                            async move {
                                fetch_bing_archive_page(&client, page, 24)
                                    .await
                                    .map(|items| (source, items))
                            },
                            |res| cosmic::Action::App(Message::OnlineMoreWallpapersFetched(res)),
                        );
                    }
                    OnlineSource::Wallhaven => {
                        self.wallhaven_page += 1;
                        let page = self.wallhaven_page;
                        let cat = self.wallhaven_category.clone();
                        let sort = self.wallhaven_sorting.clone();
                        let res = self.wallhaven_resolution.clone();
                        let search = if self.wallhaven_search.trim().is_empty() {
                            None
                        } else {
                            Some(self.wallhaven_search.clone())
                        };

                        return Task::perform(
                            async move {
                                fetch_wallhaven_wallpapers(&client, page, search.as_deref(), &cat, &sort, &res)
                                    .await
                                    .map(|items| (source, items))
                            },
                            |res| cosmic::Action::App(Message::OnlineMoreWallpapersFetched(res)),
                        );
                    }
                }
            }

            Message::OnlineMoreWallpapersFetched(result) => {
                self.explore_loading_more = false;
                match result {
                    Ok((source, items)) => {
                        let mut thumb_tasks = Vec::new();
                        let client = self.http_client.clone();

                        match source {
                            OnlineSource::Bing => {
                                for item in items {
                                    if !self.bing_wallpapers.iter().any(|e| e.id == item.id) {
                                        let thumb_dest = item.local_thumb_path();
                                        if thumb_dest.exists() {
                                            self.online_thumbs.insert(item.id.clone(), thumb_dest);
                                        } else {
                                            let c = client.clone();
                                            let id = item.id.clone();
                                            let url = item.thumb_url.clone();
                                            thumb_tasks.push(Task::perform(
                                                async move {
                                                    if let Ok(path) = download_to_file(&c, &url, &thumb_dest).await {
                                                        Some((id, path))
                                                    } else {
                                                        None
                                                    }
                                                },
                                                |res| {
                                                    if let Some((id, path)) = res {
                                                        cosmic::Action::App(Message::OnlineThumbLoaded { id, path })
                                                    } else {
                                                        cosmic::Action::None
                                                    }
                                                },
                                            ));
                                        }
                                        self.bing_wallpapers.push(item);
                                    }
                                }
                            }
                            OnlineSource::Wallhaven => {
                                for item in items {
                                    if !self.wallhaven_wallpapers.iter().any(|e| e.id == item.id) {
                                        let thumb_dest = item.local_thumb_path();
                                        if thumb_dest.exists() {
                                            self.online_thumbs.insert(item.id.clone(), thumb_dest);
                                        } else {
                                            let c = client.clone();
                                            let id = item.id.clone();
                                            let url = item.thumb_url.clone();
                                            thumb_tasks.push(Task::perform(
                                                async move {
                                                    if let Ok(path) = download_to_file(&c, &url, &thumb_dest).await {
                                                        Some((id, path))
                                                    } else {
                                                        None
                                                    }
                                                },
                                                |res| {
                                                    if let Some((id, path)) = res {
                                                        cosmic::Action::App(Message::OnlineThumbLoaded { id, path })
                                                    } else {
                                                        cosmic::Action::None
                                                    }
                                                },
                                            ));
                                        }
                                        self.wallhaven_wallpapers.push(item);
                                    }
                                }
                            }
                        }

                        if !thumb_tasks.is_empty() {
                            return Task::batch(thumb_tasks);
                        }
                    }
                    Err(err) => {
                        self.status_message = Some(format!("Error: {}", err));
                        self.status_timer = 5;
                    }
                }
            }

            Message::SelectWallhavenCategory(cat) => {
                if self.wallhaven_category != cat {
                    self.wallhaven_category = cat;
                    self.wallhaven_page = 1;
                    self.wallhaven_wallpapers.clear();
                    return Task::done(cosmic::Action::App(Message::FetchOnlineWallpapers(OnlineSource::Wallhaven)));
                }
            }

            Message::SelectWallhavenSorting(sort) => {
                if self.wallhaven_sorting != sort {
                    self.wallhaven_sorting = sort;
                    self.wallhaven_page = 1;
                    self.wallhaven_wallpapers.clear();
                    return Task::done(cosmic::Action::App(Message::FetchOnlineWallpapers(OnlineSource::Wallhaven)));
                }
            }

            Message::SelectWallhavenResolution(res) => {
                if self.wallhaven_resolution != res {
                    self.wallhaven_resolution = res;
                    self.wallhaven_page = 1;
                    self.wallhaven_wallpapers.clear();
                    return Task::done(cosmic::Action::App(Message::FetchOnlineWallpapers(OnlineSource::Wallhaven)));
                }
            }

            Message::WallhavenSearchChanged(q) => {
                self.wallhaven_search = q;
            }

            Message::SubmitWallhavenSearch => {
                self.wallhaven_page = 1;
                self.wallhaven_wallpapers.clear();
                return Task::done(cosmic::Action::App(Message::FetchOnlineWallpapers(OnlineSource::Wallhaven)));
            }

            Message::OnlineThumbLoaded { id, path } => {
                self.online_thumbs.insert(id, path);
            }

            Message::DownloadOnlineWallpaper { item, auto_apply } => {
                self.downloading_online_ids.insert(item.id.clone());
                let client = self.http_client.clone();
                let id = item.id.clone();
                let url = item.full_url.clone();
                let dest = item.local_wallpaper_path();
                let thumb_src = item.local_thumb_path();
                return Task::perform(
                    async move {
                        match download_to_file(&client, &url, &dest).await {
                            Ok(p) => {
                                if thumb_src.exists() {
                                    let target_thumb = crate::scanner::thumbs::thumb_path_for_video(&p);
                                    if let Some(parent) = target_thumb.parent() {
                                        let _ = tokio::fs::create_dir_all(parent).await;
                                    }
                                    let _ = tokio::fs::copy(&thumb_src, &target_thumb).await;
                                }
                                Ok((id, p, auto_apply))
                            }
                            Err(e) => Err((id, e)),
                        }
                    },
                    |res| match res {
                        Ok((id, path, auto_apply)) => {
                            cosmic::Action::App(Message::OnlineWallpaperDownloaded { id, path, auto_apply })
                        }
                        Err((id, error)) => {
                            cosmic::Action::App(Message::OnlineWallpaperDownloadFailed { id, error })
                        }
                    },
                );
            }

            Message::OnlineWallpaperDownloaded { id, path, auto_apply } => {
                self.downloading_online_ids.remove(&id);
                self.videos = scan_directories(&self.config.dirs, &self.config.custom_videos);
                let _ = self.config.save();
                let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let msg_text = format!("{}: {}", self.language.explore_toast_downloaded(), fname);
                let toast_task = self.notify_with_action(
                    msg_text,
                    self.language.toast_show_in_files(),
                    Message::OpenWallpapersFolder,
                );

                let mut tasks = vec![toast_task];
                if let Some(item) = self.videos.iter().find(|v| v.path == path) {
                    if item.thumb_path.is_none() {
                        let vp = path.clone();
                        tasks.push(Task::perform(
                            async move { (vp.clone(), generate_thumbnail(&vp).await) },
                            move |(video_path, t)| {
                                if let Some(thumb_path) = t {
                                    cosmic::Action::App(Message::ThumbnailGenerated { video_path, thumb_path })
                                } else {
                                    cosmic::Action::None
                                }
                            },
                        ));
                    }
                }

                if auto_apply {
                    tasks.push(Task::done(cosmic::Action::App(Message::ApplyDownloadedOnlineWallpaper(path))));
                }

                if !tasks.is_empty() {
                    return Task::batch(tasks);
                }
            }

            Message::OnlineWallpaperDownloadFailed { id, error } => {
                self.downloading_online_ids.remove(&id);
                self.status_message = Some(format!("{} {}", self.language.explore_error_prefix(), error));
                self.status_timer = 5;
            }

            Message::ApplyDownloadedOnlineWallpaper(path) => {
                let output = self.selected_output.clone();
                return Task::done(cosmic::Action::App(Message::ApplyWallpaper {
                    video_path: path,
                    output,
                }));
            }

            Message::RemoveWallpaperFromLibrary(path) => {
                let path_str = path.to_string_lossy().to_string();

                let was_active = self.config.current.as_deref() == Some(&path_str)
                    || self.config.wallpapers.values().any(|p| p == &path_str);

                if was_active {
                    let mut outputs_to_stop = Vec::new();
                    for (out, p) in &self.config.wallpapers {
                        if p == &path_str {
                            outputs_to_stop.push(out.clone());
                        }
                    }
                    for out in outputs_to_stop {
                        self.engine.stop_output(&out);
                        self.config.wallpapers.remove(&out);
                    }
                    if self.config.current.as_deref() == Some(&path_str) {
                        self.config.current = None;
                    }
                }

                // Remove from custom_videos (does NOT delete user's file from disk!)
                self.config.custom_videos.retain(|p| p != &path_str);
                let _ = self.config.save();
                self.videos = scan_directories(&self.config.dirs, &self.config.custom_videos);

                return self.set_status(self.language.library_removed_toast());
            }

            Message::DeleteDownloadedWallpaper(path) => {
                let path_str = path.to_string_lossy().to_string();

                let was_active = self.config.current.as_deref() == Some(&path_str)
                    || self.config.wallpapers.values().any(|p| p == &path_str);

                if was_active {
                    let mut outputs_to_stop = Vec::new();
                    for (out, p) in &self.config.wallpapers {
                        if p == &path_str {
                            outputs_to_stop.push(out.clone());
                        }
                    }
                    for out in outputs_to_stop {
                        self.engine.stop_output(&out);
                        self.config.wallpapers.remove(&out);
                    }
                    if self.config.current.as_deref() == Some(&path_str) {
                        self.config.current = None;
                    }
                }

                // Safe check: Only physically delete from disk if located inside the online downloads directory
                let online_dir = crate::online::wallpapers_online_dir();
                if path.starts_with(&online_dir) && path.is_file() {
                    let _ = std::fs::remove_file(&path);
                }

                self.videos = scan_directories(&self.config.dirs, &self.config.custom_videos);

                let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                return self.set_status(format!("{}: {}", self.language.library_deleted_toast(), fname));
            }

            Message::DeleteWallpaper(path) => {
                let online_dir = crate::online::wallpapers_online_dir();
                if path.starts_with(&online_dir) {
                    return Task::done(cosmic::Action::App(Message::DeleteDownloadedWallpaper(path)));
                } else {
                    return Task::done(cosmic::Action::App(Message::RemoveWallpaperFromLibrary(path)));
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let content: Element<_> = match self.active_page {
            Page::Library => self.view_library(),
            Page::Explore => self.view_explore(),
            Page::Monitors => self.view_monitors(),
            Page::Settings => self.view_settings(),
            Page::About => self.view_about(),
        };

        let mut main_col = widget::column::with_capacity(2)
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill);

        main_col = main_col.push(content);

        // Fixed Global "Now Playing" Bottom Bar
        main_col = main_col.push(self.view_now_playing_bar());

        let main_container = widget::container(main_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(14);

        Element::from(widget::toaster(&self.toasts, main_container))
    }
}

impl AuraApp {
    pub fn build_nav(lang: crate::i18n::Language, active_page: Page) -> nav_bar::Model {
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text(lang.nav_library())
            .data(Page::Library)
            .icon(widget::icon::from_name("video-x-generic-symbolic"));

        nav.insert()
            .text(lang.nav_explore())
            .data(Page::Explore)
            .icon(widget::icon::from_name("web-browser-symbolic"));

        nav.insert()
            .text(lang.nav_monitors())
            .data(Page::Monitors)
            .icon(widget::icon::from_name("video-display-symbolic"));

        nav.insert()
            .text(lang.nav_settings())
            .data(Page::Settings)
            .icon(widget::icon::from_name("preferences-system-symbolic"));

        nav.insert()
            .text(lang.nav_about())
            .data(Page::About)
            .icon(widget::icon::from_name("help-about-symbolic"));

        match active_page {
            Page::Library => { nav.activate_position(0); }
            Page::Explore => { nav.activate_position(1); }
            Page::Monitors => { nav.activate_position(2); }
            Page::Settings => { nav.activate_position(3); }
            Page::About => { nav.activate_position(4); }
        }

        nav
    }
}
