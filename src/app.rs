use crate::config::Config;
use crate::engine::{detect_outputs, MonitorOutput, WallpaperEngine};
use crate::scanner::{scan_directories, thumbs::generate_thumbnail, VideoItem, VIDEO_EXTENSIONS};
use crate::theme::apply_cosmic_theme;
use crate::online::{
    download_to_file, fetch_bing_wallpapers, fetch_wallhaven_wallpapers,
    OnlineSource, OnlineWallpaperItem,
};

use cosmic::app::Core;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Subscription, Task};
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

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    SelectPage(nav_bar::Id),
    SetLanguage(crate::i18n::Language),
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
    TickSecond,
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
    TrayPoll,
    ShowMainWindow,
    WindowCloseRequested(cosmic::iced::window::Id),
    WindowClosed(cosmic::iced::window::Id),
    NextWallpaper,
    QuitApp,
    SelectExploreSource(OnlineSource),
    FetchOnlineWallpapers(OnlineSource),
    OnlineWallpapersFetched(Result<(OnlineSource, Vec<OnlineWallpaperItem>), String>),
    DownloadOnlineWallpaper { item: OnlineWallpaperItem, auto_apply: bool },
    OnlineWallpaperDownloaded { id: String, path: PathBuf, auto_apply: bool },
    OnlineWallpaperDownloadFailed { id: String, error: String },
    OnlineThumbLoaded { id: String, path: PathBuf },
    ApplyDownloadedOnlineWallpaper(PathBuf),
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
pub struct AuraFlags;

impl cosmic::app::CosmicFlags for AuraFlags {
    type SubCommand = String;
    type Args = Vec<String>;
}

pub struct AuraApp {
    core: Core,
    nav: nav_bar::Model,
    active_page: Page,
    config: Config,
    pub language: crate::i18n::Language,
    engine: WallpaperEngine,
    outputs: Vec<MonitorOutput>,
    selected_output: String,
    videos: Vec<VideoItem>,
    search_query: String,
    autostart_active: bool,
    status_message: Option<String>,
    status_timer: u8,
    is_paused: bool,
    tray_controller: crate::tray::TrayController,
    is_window_open: bool,
    explore_source: OnlineSource,
    bing_wallpapers: Vec<OnlineWallpaperItem>,
    wallhaven_wallpapers: Vec<OnlineWallpaperItem>,
    explore_loading: bool,
    explore_error: Option<String>,
    downloading_online_ids: std::collections::HashSet<String>,
    online_thumbs: std::collections::HashMap<String, PathBuf>,
    http_client: reqwest::Client,
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

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let config = Config::load();
        let language = crate::i18n::Language::from_code(&config.language);
        let nav = Self::build_nav(language, Page::Library);

        let outputs = detect_outputs();
        let selected_output = outputs.first().map(|o| o.name.clone()).unwrap_or_else(|| "*".into());
        let autostart_active = WallpaperEngine::is_autostart_enabled();
        let videos = scan_directories(&config.dirs, &config.custom_videos);

        // Spawn async background thumbnail generation
        let mut tasks = Vec::new();
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

        let app = Self {
            core,
            nav,
            active_page: Page::Library,
            config,
            language,
            engine: WallpaperEngine::new(),
            outputs,
            selected_output,
            videos,
            search_query: String::new(),
            autostart_active,
            status_message: None,
            status_timer: 0,
            is_paused: false,
            tray_controller,
            is_window_open: true,
            explore_source: OnlineSource::Bing,
            bing_wallpapers: Vec::new(),
            wallhaven_wallpapers: Vec::new(),
            explore_loading: false,
            explore_error: None,
            downloading_online_ids: std::collections::HashSet::new(),
            online_thumbs: std::collections::HashMap::new(),
            http_client: reqwest::Client::new(),
        };

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

    fn dbus_activation(&mut self, _msg: cosmic::dbus_activation::Message) -> Task<cosmic::Action<Self::Message>> {
        Task::done(cosmic::Action::App(Message::ShowMainWindow))
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
                    .on_press(Message::PickVideoFile)
            ),
            Element::from(
                widget::button::standard(self.language.header_add_folder())
                    .on_press(Message::PickFolder)
            ),
        ]
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let mut subs = Vec::new();

        // 1. Drag & Drop subscription via filtered listen_with
        subs.push(cosmic::iced::event::listen_with(handle_window_events));

        // 2. 1-second interval timer for auto-dismissing notifications
        subs.push(
            cosmic::iced::time::every(Duration::from_secs(1))
                .map(|_| Message::TickSecond)
        );

        // 3. Tray actions listener (every 120ms, non-blocking)
        subs.push(
            cosmic::iced::time::every(Duration::from_millis(120))
                .map(|_| Message::TrayPoll)
        );

        // 3. Playlist auto-rotation
        if self.config.rotation && self.config.interval > 0 {
            subs.push(
                cosmic::iced::time::every(Duration::from_secs(self.config.interval * 60))
                    .map(|_| Message::RotationTick)
            );
        }

        // 4. Display hotplug monitor detector (every 5 seconds)
        subs.push(
            cosmic::iced::time::every(Duration::from_secs(5))
                .map(|_| Message::HotplugTick)
        );

        // 5. Smart Pause: Game & Fullscreen detection (every 3 seconds)
        if self.config.smart_pause && !self.config.wallpapers.is_empty() {
            subs.push(
                cosmic::iced::time::every(Duration::from_secs(3))
                    .map(|_| Message::SmartPauseTick)
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

            Message::TrayPoll => {
                while let Ok(action) = self.tray_controller.rx.try_recv() {
                    match action {
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
                    return open_task.discard();
                } else if let Some(id) = self.core().main_window_id() {
                    return cosmic::iced::window::gain_focus(id);
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

            Message::SetLanguage(lang) => {
                self.language = lang;
                self.config.language = lang.code().to_string();
                let _ = self.config.save();
                self.nav = Self::build_nav(self.language, self.active_page);
                self.tray_controller.set_language(lang);
                self.status_message = Some(lang.status_lang_changed().into());
                self.status_timer = 5;
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

            Message::ApplyWallpaper { video_path, output } => {
                let scaling = self.config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());
                let mute = self.config.mute;
                let hwdec = self.config.hwdec.clone();

                let path_str = video_path.to_string_lossy().to_string();
                if let Ok(_) = self.engine.set_wallpaper(&output, &path_str, &scaling, mute, &hwdec) {
                    self.config.wallpapers.insert(output.clone(), path_str.clone());
                    self.config.current = Some(path_str.clone());
                    let _ = self.config.save();
                    self.is_paused = false;

                    let file_name = video_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Video".into());
                    self.status_message = Some(self.language.status_applied(&output, &file_name));
                    self.status_timer = 5;

                    self.tray_controller.update_state(file_name, false, true);

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

                    if self.autostart_active {
                        let _ = self.engine.write_autostart(&self.config.wallpapers, &self.config.scaling, mute, &hwdec);
                    }
                }
            }

            Message::TogglePause => {
                let now_paused = self.engine.toggle_pause();
                self.is_paused = now_paused;
                self.status_message = Some(if now_paused {
                    self.language.status_paused().into()
                } else {
                    self.language.status_resumed().into()
                });
                self.status_timer = 5;

                let current_title = self.config.current.as_ref()
                    .and_then(|c| std::path::Path::new(c).file_stem().map(|s| s.to_string_lossy().to_string()))
                    .unwrap_or_default();
                self.tray_controller.update_state(current_title, now_paused, !self.config.wallpapers.is_empty());
            }

            Message::StopWallpaper(output) => {
                if let Some(out) = output {
                    self.engine.stop_output(&out);
                    self.config.wallpapers.remove(&out);
                    self.status_message = Some(self.language.status_stopped_output(&out));
                } else {
                    self.engine.stop_all();
                    self.config.wallpapers.clear();
                    self.config.current = None;
                    self.status_message = Some(self.language.status_stopped_all().into());
                }
                self.status_timer = 5;
                self.is_paused = false;
                let _ = self.config.save();
                if self.autostart_active {
                    let _ = self.engine.write_autostart(&self.config.wallpapers, &self.config.scaling, self.config.mute, &self.config.hwdec);
                }

                let current_title = self.config.current.as_ref()
                    .and_then(|c| std::path::Path::new(c).file_stem().map(|s| s.to_string_lossy().to_string()))
                    .unwrap_or_default();
                self.tray_controller.update_state(current_title, false, !self.config.wallpapers.is_empty());
            }

            Message::SelectScaling { output, scaling } => {
                self.config.scaling.insert(output.clone(), scaling.clone());
                let _ = self.config.save();
                if let Some(path) = self.config.wallpapers.get(&output).cloned() {
                    let _ = self.engine.set_wallpaper(&output, &path, &scaling, self.config.mute, &self.config.hwdec);
                    self.status_message = Some(format!("Modo de escala para {} cambiado a '{}'", output, scaling));
                    self.status_timer = 5;
                }
            }

            Message::ToggleAutostart(active) => {
                self.autostart_active = active;
                if active {
                    let _ = self.engine.write_autostart(&self.config.wallpapers, &self.config.scaling, self.config.mute, &self.config.hwdec);
                    self.status_message = Some("Inicio automático activado".into());
                } else {
                    let _ = WallpaperEngine::remove_autostart();
                    self.status_message = Some("Inicio automático desactivado".into());
                }
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
                    let _ = self.engine.set_wallpaper(&output, &path, &sc, mute, &self.config.hwdec);
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

            // XDG File Dialog: Pick Video
            Message::PickVideoFile => {
                let title = self.language.dialog_pick_video();
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title(title)
                            .add_filter("Videos", &["mp4", "webm", "mkv", "avi", "mov"])
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

                // Trigger thumbnail generation and apply immediately
                let v_path = path.clone();
                let output = self.selected_output.clone();
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

            // XDG File Dialog: Pick Folder
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

                // Generate thumbnails for new videos in folder
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

            // Drag & Drop
            Message::FilesDropped(paths) => {
                let mut added_any = false;
                let mut apply_last: Option<PathBuf> = None;

                for p in paths {
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        if VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
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

                    if let Some(video_to_apply) = apply_last {
                        let out = self.selected_output.clone();
                        let vp = video_to_apply.clone();
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
                    let is_gaming = check_if_fullscreen_game_active();
                    if is_gaming && !self.is_paused {
                        self.engine.pause_all();
                        self.is_paused = true;
                        self.status_message = Some(self.language.status_game_paused().into());
                        self.status_timer = 5;
                    } else if !is_gaming && self.is_paused {
                        self.engine.resume_all();
                        self.is_paused = false;
                        self.status_message = Some(self.language.status_game_resumed().into());
                        self.status_timer = 5;
                    }
                }
            }

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
                return Task::perform(
                    async move {
                        match source {
                            OnlineSource::Bing => {
                                fetch_bing_wallpapers(&client)
                                    .await
                                    .map(|items| (source, items))
                            }
                            OnlineSource::Wallhaven => {
                                fetch_wallhaven_wallpapers(&client)
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

                        // Preload thumbnails in background if not already cached
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

            Message::OnlineThumbLoaded { id, path } => {
                self.online_thumbs.insert(id, path);
            }

            Message::DownloadOnlineWallpaper { item, auto_apply } => {
                self.downloading_online_ids.insert(item.id.clone());
                let client = self.http_client.clone();
                let id = item.id.clone();
                let url = item.full_url.clone();
                let dest = item.local_wallpaper_path();
                return Task::perform(
                    async move {
                        match download_to_file(&client, &url, &dest).await {
                            Ok(p) => Ok((id, p, auto_apply)),
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
                // Rescan library to immediately include this wallpaper in the local catalog
                self.videos = scan_directories(&self.config.dirs, &self.config.custom_videos);
                let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                self.status_message = Some(format!("{} ({})", self.language.explore_toast_downloaded(), fname));
                self.status_timer = 5;

                if auto_apply {
                    return Task::done(cosmic::Action::App(Message::ApplyDownloadedOnlineWallpaper(path)));
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

        let mut main_col = widget::column::with_capacity(3)
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill);

        // Dismissible notification banner (disappears automatically after 5s or clicking ✕)
        if let Some(msg) = &self.status_message {
            let banner = widget::container(
                widget::row::with_capacity(2)
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .push(widget::text::body(msg).size(13).width(Length::Fill))
                    .push(
                        widget::button::standard("✕")
                            .on_press(Message::DismissStatus)
                    )
            )
            .padding(10)
            .width(Length::Fill);
            main_col = main_col.push(banner);
        }

        main_col = main_col.push(content);

        // Fixed Global "Now Playing" Bottom Bar
        main_col = main_col.push(self.view_now_playing_bar());

        Element::from(
            widget::container(main_col)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(14)
        )
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

    fn view_library(&self) -> Element<'_, Message> {
        let search_bar = widget::text_input::search_input(self.language.library_search_placeholder(), &self.search_query)
            .on_input(Message::SearchChanged)
            .width(Length::Fill);

        let filtered_videos: Vec<VideoItem> = self.videos
            .iter()
            .filter(|v| {
                if self.search_query.trim().is_empty() {
                    true
                } else {
                    v.name.to_lowercase().contains(&self.search_query.to_lowercase())
                }
            })
            .cloned()
            .collect();

        if filtered_videos.is_empty() {
            let empty_msg = widget::column::with_capacity(4)
                .spacing(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title2(self.language.library_empty_title()))
                .push(widget::text::body(self.language.library_empty_desc()))
                .push(
                    widget::row::with_capacity(2)
                        .spacing(12)
                        .push(widget::button::suggested(self.language.library_btn_add_video()).on_press(Message::PickVideoFile))
                        .push(widget::button::standard(self.language.library_btn_add_folder()).on_press(Message::PickFolder))
                );

            return Element::from(
                widget::container(empty_msg)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
            );
        }

        let selected_output = self.selected_output.clone();
        let active_wallpapers = self.config.wallpapers.clone();

        // Responsive reflow grid adapting dynamically to window resize!
        let grid = widget::responsive(move |size| {
            let card_w = 264.0f32;
            let gap = 16.0f32;
            let available_w = size.width.max(280.0);
            let cols = ((available_w + gap) / (card_w + gap)).floor().max(1.0) as usize;

            let mut cards_column = widget::column::with_capacity(filtered_videos.len() / cols + 1)
                .spacing(gap)
                .width(Length::Fill);

            for chunk in filtered_videos.chunks(cols) {
                let mut card_row = widget::row::with_capacity(chunk.len()).spacing(gap);
                for video in chunk {
                    let is_active = active_wallpapers.values().any(|p| p == &video.path.to_string_lossy());

                    let mut card_content = widget::column::with_capacity(5).spacing(8).padding(12);

                    if let Some(thumb) = &video.thumb_path {
                        let img_btn = widget::button::image(thumb.to_string_lossy().to_string())
                            .width(240.0)
                            .selected(is_active)
                            .on_press(Message::ApplyWallpaper {
                                video_path: video.path.clone(),
                                output: selected_output.clone(),
                            });
                        card_content = card_content.push(img_btn);
                    } else {
                        let placeholder = widget::container(widget::text::body(self.language.library_extracting_frame()))
                            .width(Length::Fixed(240.0))
                            .height(Length::Fixed(135.0))
                            .align_x(Horizontal::Center)
                            .align_y(Vertical::Center);
                        card_content = card_content.push(placeholder);
                    }

                    let title = widget::text::body(video.name.clone()).size(14);
                    let size_lbl = widget::text::caption(video.size_formatted.clone());
                    card_content = card_content.push(title).push(size_lbl);

                    let apply_btn = if is_active {
                        widget::button::suggested(self.language.library_active())
                    } else {
                        widget::button::standard(self.language.library_apply())
                    }.on_press(Message::ApplyWallpaper {
                        video_path: video.path.clone(),
                        output: selected_output.clone(),
                    });

                    card_content = card_content.push(apply_btn);

                    let card_container = widget::container(card_content)
                        .width(Length::Fixed(card_w));

                    card_row = card_row.push(card_container);
                }
                cards_column = cards_column.push(card_row);
            }

            Element::from(cards_column)
        });

        let scroll = widget::scrollable(grid).height(Length::Fill);

        Element::from(
            widget::column::with_capacity(2)
                .spacing(14)
                .push(search_bar)
                .push(scroll)
        )
    }

    fn view_explore(&self) -> Element<'_, Message> {
        let lang = self.language;
        let active_wallpapers = self.config.wallpapers.clone();
        let downloading_ids = self.downloading_online_ids.clone();
        let online_thumbs = self.online_thumbs.clone();
        let current_source = self.explore_source;

        let build_header = || {
            let bing_btn = if current_source == OnlineSource::Bing {
                widget::button::suggested(lang.explore_source_bing())
            } else {
                widget::button::standard(lang.explore_source_bing())
            }.on_press(Message::SelectExploreSource(OnlineSource::Bing));

            let wallhaven_btn = if current_source == OnlineSource::Wallhaven {
                widget::button::suggested(lang.explore_source_wallhaven())
            } else {
                widget::button::standard(lang.explore_source_wallhaven())
            }.on_press(Message::SelectExploreSource(OnlineSource::Wallhaven));

            let reload_btn = widget::button::standard("🔄")
                .on_press(Message::FetchOnlineWallpapers(current_source));

            let top_bar = widget::row::with_capacity(3)
                .spacing(12)
                .align_y(Alignment::Center)
                .push(bing_btn)
                .push(wallhaven_btn)
                .push(reload_btn);

            let subtitle = match current_source {
                OnlineSource::Bing => lang.explore_featured_today(),
                OnlineSource::Wallhaven => lang.explore_recent_title(),
            };

            widget::column::with_capacity(2)
                .spacing(6)
                .push(top_bar)
                .push(widget::text::caption(subtitle))
        };

        if self.explore_loading {
            let loading_content = widget::column::with_capacity(3)
                .spacing(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title2(lang.explore_loading()))
                .push(widget::text::caption(match current_source {
                    OnlineSource::Bing => "Bing Daily Wallpaper UHD (4K)",
                    OnlineSource::Wallhaven => "Wallhaven Anime & Nature 4K",
                }));

            let loading_view = widget::container(loading_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            return Element::from(
                widget::column::with_capacity(2)
                    .spacing(14)
                    .push(build_header())
                    .push(loading_view)
            );
        }

        if let Some(err) = &self.explore_error {
            let error_content = widget::column::with_capacity(3)
                .spacing(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title3(format!("{} {}", lang.explore_error_prefix(), err)))
                .push(
                    widget::button::suggested(lang.explore_btn_retry())
                        .on_press(Message::FetchOnlineWallpapers(current_source))
                );

            let error_view = widget::container(error_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            return Element::from(
                widget::column::with_capacity(2)
                    .spacing(14)
                    .push(build_header())
                    .push(error_view)
            );
        }

        let items: Vec<OnlineWallpaperItem> = match current_source {
            OnlineSource::Bing => self.bing_wallpapers.clone(),
            OnlineSource::Wallhaven => self.wallhaven_wallpapers.clone(),
        };

        if items.is_empty() {
            let empty_content = widget::column::with_capacity(3)
                .spacing(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title3(lang.explore_loading()))
                .push(
                    widget::button::suggested(lang.explore_btn_retry())
                        .on_press(Message::FetchOnlineWallpapers(current_source))
                );

            let empty_view = widget::container(empty_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            return Element::from(
                widget::column::with_capacity(2)
                    .spacing(14)
                    .push(build_header())
                    .push(empty_view)
            );
        }

        let grid = widget::responsive(move |size| {
            let card_w = 264.0f32;
            let gap = 16.0f32;
            let available_w = size.width.max(280.0);
            let cols = ((available_w + gap) / (card_w + gap)).floor().max(1.0) as usize;

            let mut cards_column = widget::column::with_capacity(items.len() / cols + 1)
                .spacing(gap)
                .width(Length::Fill);

            for chunk in items.chunks(cols) {
                let mut card_row = widget::row::with_capacity(chunk.len()).spacing(gap);
                for item in chunk {
                    let is_downloading = downloading_ids.contains(&item.id);
                    let downloaded_path = item.is_downloaded();
                    let is_active = if let Some(dp) = &downloaded_path {
                        active_wallpapers.values().any(|p| p == &dp.to_string_lossy())
                    } else {
                        false
                    };

                    let thumb_path = online_thumbs.get(&item.id).cloned().or_else(|| {
                        let p = item.local_thumb_path();
                        if p.exists() { Some(p) } else { None }
                    });

                    let mut card_content = widget::column::with_capacity(6).spacing(8).padding(12);

                    // Image preview button
                    if let Some(thumb) = &thumb_path {
                        let img_press = if let Some(dp) = &downloaded_path {
                            Message::ApplyDownloadedOnlineWallpaper(dp.clone())
                        } else {
                            Message::DownloadOnlineWallpaper {
                                item: item.clone(),
                                auto_apply: true,
                            }
                        };

                        let img_btn = widget::button::image(thumb.to_string_lossy().to_string())
                            .width(240.0)
                            .selected(is_active)
                            .on_press(img_press);
                        card_content = card_content.push(img_btn);
                    } else {
                        let placeholder = widget::container(widget::text::caption(lang.explore_loading()))
                            .width(Length::Fixed(240.0))
                            .height(Length::Fixed(135.0))
                            .align_x(Horizontal::Center)
                            .align_y(Vertical::Center);
                        card_content = card_content.push(placeholder);
                    }

                    // Title
                    let title = widget::text::body(item.title.clone()).size(14);
                    card_content = card_content.push(title);

                    // Metadata line: Resolution • Author/Date
                    let meta_text = if let Some(d) = &item.date {
                        format!("{} • {}", item.resolution, d)
                    } else if !item.author_or_copyright.is_empty() {
                        format!("{} • {}", item.resolution, item.author_or_copyright)
                    } else {
                        item.resolution.clone()
                    };
                    let meta_lbl = widget::text::caption(meta_text);
                    card_content = card_content.push(meta_lbl);

                    // Morphing Action Button
                    let action_btn = if is_downloading {
                        widget::button::standard(lang.explore_btn_downloading())
                    } else if is_active {
                        widget::button::suggested(lang.explore_badge_active())
                    } else if let Some(dp) = downloaded_path {
                        widget::button::suggested(lang.explore_btn_apply())
                            .on_press(Message::ApplyDownloadedOnlineWallpaper(dp))
                    } else {
                        widget::button::standard(lang.explore_btn_download())
                            .on_press(Message::DownloadOnlineWallpaper {
                                item: item.clone(),
                                auto_apply: false,
                            })
                    };

                    card_content = card_content.push(action_btn);

                    let card_container = widget::container(card_content)
                        .width(Length::Fixed(card_w));

                    card_row = card_row.push(card_container);
                }
                cards_column = cards_column.push(card_row);
            }

            Element::from(cards_column)
        });

        let scroll = widget::scrollable(grid).height(Length::Fill);

        Element::from(
            widget::column::with_capacity(2)
                .spacing(14)
                .push(build_header())
                .push(scroll)
        )
    }

    fn view_monitors(&self) -> Element<'_, Message> {
        let mut col = widget::column::with_capacity(self.outputs.len() + 3)
            .spacing(18)
            .width(Length::Fill);

        col = col.push(widget::text::title2(self.language.monitors_title()));
        col = col.push(widget::text::body(self.language.monitors_desc()));

        // Horizontal Canvas of Virtual Monitors
        let mut monitors_canvas = widget::row::with_capacity(self.outputs.len())
            .spacing(24)
            .align_y(Alignment::End);

        for monitor in &self.outputs {
            let active_wall = self.config.wallpapers.get(&monitor.name);
            let active_scaling = self.config.scaling.get(&monitor.name).map(|s| s.as_str()).unwrap_or("fit");

            let screen_w = 260.0f32;

            // Screen frame representation
            let screen_display: Element<'_, Message> = if let Some(wall_path) = active_wall {
                let thumb = crate::scanner::thumbs::thumb_path_for_video(std::path::Path::new(wall_path));
                if thumb.exists() {
                    Element::from(
                        widget::button::image(thumb.to_string_lossy().to_string())
                            .width(screen_w)
                            .selected(true)
                    )
                } else {
                    Element::from(
                        widget::button::standard(self.language.monitors_screen_name(&monitor.name))
                            .width(screen_w)
                    )
                }
            } else {
                Element::from(
                    widget::button::standard(self.language.monitors_screen_idle(&monitor.name))
                        .width(screen_w)
                )
            };

            let monitor_card = widget::column::with_capacity(6)
                .spacing(10)
                .padding(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title3(format!("{} • {}", monitor.name, monitor.resolution)))
                .push(screen_display)
                .push(
                    widget::row::with_capacity(4)
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(
                            if active_scaling == "fit" {
                                widget::button::suggested("Fit")
                            } else {
                                widget::button::standard("Fit")
                            }.on_press(Message::SelectScaling { output: monitor.name.clone(), scaling: "fit".into() })
                        )
                        .push(
                            if active_scaling == "fill" {
                                widget::button::suggested("Fill")
                            } else {
                                widget::button::standard("Fill")
                            }.on_press(Message::SelectScaling { output: monitor.name.clone(), scaling: "fill".into() })
                        )
                        .push(
                            if active_scaling == "stretch" {
                                widget::button::suggested("Stretch")
                            } else {
                                widget::button::standard("Stretch")
                            }.on_press(Message::SelectScaling { output: monitor.name.clone(), scaling: "stretch".into() })
                        )
                        .push(
                            widget::button::destructive(self.language.monitors_stop())
                                .on_press(Message::StopWallpaper(Some(monitor.name.clone())))
                        )
                );

            monitors_canvas = monitors_canvas.push(widget::container(monitor_card));
        }

        col = col.push(widget::scrollable(monitors_canvas));

        Element::from(col)
    }

    fn view_now_playing_bar(&self) -> Element<'_, Message> {
        let has_wallpapers = !self.config.wallpapers.is_empty();

        let (wall_title, out_label) = if let Some(curr) = &self.config.current {
            let p = std::path::Path::new(curr);
            let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| self.language.bar_active_title_default().into());
            let out = self.config.wallpapers.keys().cloned().collect::<Vec<_>>().join(", ");
            (name, self.language.bar_display_label(&out))
        } else {
            (self.language.bar_idle_title().into(), self.language.bar_idle_desc().into())
        };

        let play_pause_btn = if self.is_paused {
            widget::button::suggested(self.language.bar_resume()).on_press_maybe(if has_wallpapers { Some(Message::TogglePause) } else { None })
        } else {
            widget::button::standard(self.language.bar_pause()).on_press_maybe(if has_wallpapers { Some(Message::TogglePause) } else { None })
        };

        let mute_btn = if self.config.mute {
            widget::button::standard(self.language.bar_muted()).on_press(Message::ToggleMute(false))
        } else {
            widget::button::suggested(self.language.bar_audio_active()).on_press(Message::ToggleMute(true))
        };

        let stop_btn = widget::button::destructive(self.language.bar_stop())
            .on_press_maybe(if has_wallpapers { Some(Message::StopWallpaper(None)) } else { None });

        let bar_content = widget::row::with_capacity(3)
            .spacing(20)
            .align_y(Alignment::Center)
            .width(Length::Fill)
            // Left: Current title
            .push(
                widget::column::with_capacity(2)
                    .spacing(2)
                    .width(Length::Fill)
                    .push(widget::text::body(wall_title).size(14))
                    .push(widget::text::caption(out_label))
            )
            // Center: Playback Controls
            .push(
                widget::row::with_capacity(3)
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .push(play_pause_btn)
                    .push(mute_btn)
                    .push(stop_btn)
            )
            // Right: GPU / HWDEC badge
            .push(
                widget::column::with_capacity(2)
                    .spacing(2)
                    .align_x(Horizontal::Right)
                    .push(widget::text::caption(self.language.bar_gpu_accel(&self.config.hwdec)))
                    .push(widget::text::caption(self.language.bar_wayland_tag()))
            );

        Element::from(
            widget::container(bar_content)
                .padding(12)
                .width(Length::Fill)
        )
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let mut col = widget::column::with_capacity(5)
            .spacing(18)
            .width(Length::Fill);

        col = col.push(widget::text::title2(self.language.settings_title()));

        // Language selection section
        let lang_section = widget::column::with_capacity(3)
            .spacing(12)
            .padding(16)
            .push(widget::text::title3(self.language.settings_lang_title()))
            .push(widget::text::caption(self.language.settings_lang_desc()))
            .push(
                widget::row::with_capacity(2)
                    .spacing(12)
                    .push(
                        if self.language == crate::i18n::Language::Es {
                            widget::button::suggested("Español")
                        } else {
                            widget::button::standard("Español")
                        }
                        .on_press(Message::SetLanguage(crate::i18n::Language::Es))
                    )
                    .push(
                        if self.language == crate::i18n::Language::En {
                            widget::button::suggested("English")
                        } else {
                            widget::button::standard("English")
                        }
                        .on_press(Message::SetLanguage(crate::i18n::Language::En))
                    )
            );

        col = col.push(widget::container(lang_section).width(Length::Fill));

        // General settings (Clean, elegant, emoji-free)
        let general_section = widget::column::with_capacity(6)
            .spacing(14)
            .padding(16)
            .push(widget::text::title3(self.language.settings_integration_title()))
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.autostart_active).on_toggle(Message::ToggleAutostart))
                    .push(widget::text::body(self.language.settings_autostart()))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.smart_pause).on_toggle(Message::ToggleSmartPause))
                    .push(widget::text::body(self.language.settings_smart_pause()))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.keep_running_on_close).on_toggle(Message::ToggleKeepRunningOnClose))
                    .push(widget::text::body(self.language.settings_keep_running()))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.auto_theme).on_toggle(Message::ToggleAutoTheme))
                    .push(widget::text::body(self.language.settings_auto_theme()))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.auto_dark).on_toggle(Message::ToggleAutoDark))
                    .push(widget::text::body(self.language.settings_auto_dark()))
            );

        col = col.push(widget::container(general_section).width(Length::Fill));

        // Folders section
        let mut folders_box = widget::column::with_capacity(self.config.dirs.len() + 2)
            .spacing(12)
            .padding(16);

        folders_box = folders_box.push(
            widget::row::with_capacity(2)
                .spacing(16)
                .align_y(Alignment::Center)
                .push(widget::text::title3(self.language.settings_monitored_folders()).width(Length::Fill))
                .push(widget::button::suggested(self.language.settings_btn_add_folder()).on_press(Message::PickFolder))
        );

        for dir in &self.config.dirs {
            let dir_clone = dir.clone();
            let dir_row = widget::row::with_capacity(2)
                .spacing(12)
                .align_y(Alignment::Center)
                .push(widget::text::body(dir).width(Length::Fill))
                .push(
                    widget::button::destructive(self.language.settings_btn_delete())
                        .on_press(Message::RemoveFolder(dir_clone))
                );
            folders_box = folders_box.push(dir_row);
        }

        col = col.push(widget::container(folders_box).width(Length::Fill));

        // Persistence info card (clean, no emojis)
        let info_card = widget::column::with_capacity(2)
            .spacing(6)
            .padding(14)
            .push(widget::text::title3(self.language.settings_persistence_title()))
            .push(widget::text::caption(self.language.settings_persistence_desc()));

        col = col.push(widget::container(info_card).width(Length::Fill));

        Element::from(widget::scrollable(col).height(Length::Fill))
    }

    fn view_about(&self) -> Element<'_, Message> {
        let mut col = widget::column::with_capacity(6)
            .spacing(18)
            .padding(24)
            .align_x(Horizontal::Center)
            .width(Length::Fill);

        // Official high-resolution Kinetic A vector logo
        let logo_widget = widget::icon::from_svg_bytes(
            include_bytes!("../resources/icons/hicolor/scalable/apps/io.github.antwny.aura.svg")
        )
        .icon()
        .size(104);

        let title_box = widget::column::with_capacity(3)
            .spacing(6)
            .align_x(Horizontal::Center)
            .push(widget::text::title1("Aura"))
            .push(widget::text::title3(self.language.about_tagline()))
            .push(widget::text::caption(self.language.about_version_info()));

        let info_card = widget::column::with_capacity(7)
            .spacing(12)
            .padding(20)
            .width(Length::Fixed(580.0))
            .push(widget::text::title3(self.language.about_details_title()))
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .push(widget::text::body(self.language.about_developer_lbl()).width(Length::Fixed(140.0)))
                    .push(widget::text::body("Antwny"))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .push(widget::text::body(self.language.about_donation_lbl()).width(Length::Fixed(140.0)))
                    .push(widget::text::body("antwnyab@gmail.com"))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .push(widget::text::body(self.language.about_architecture_lbl()).width(Length::Fixed(140.0)))
                    .push(widget::text::caption("Rust 1.95 • libcosmic • wgpu • Tokio • mpvpaper"))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .push(widget::text::body(self.language.about_license_lbl()).width(Length::Fixed(140.0)))
                    .push(widget::text::body("GNU General Public License v3.0 (GPL-3.0)"))
            )
            .push(
                widget::text::caption(self.language.about_summary_desc())
            );

        let action_buttons = widget::row::with_capacity(3)
            .spacing(14)
            .align_y(Alignment::Center)
            .push(
                widget::button::suggested(self.language.about_github_btn())
                    .on_press(Message::OpenGitHub)
            )
            .push(
                widget::button::standard(self.language.about_youtube_btn())
                    .on_press(Message::OpenYouTube)
            )
            .push(
                widget::button::standard(self.language.about_donate_btn())
                    .on_press(Message::OpenPayPal)
            );

        col = col.push(logo_widget)
            .push(title_box)
            .push(widget::container(info_card))
            .push(action_buttons);

        Element::from(widget::scrollable(col).height(Length::Fill))
    }
}

pub fn is_heavy_process_cmdline(cmdline: &str) -> bool {
    const HEAVY_PROCESSES: &[&str] = &["gamescope", "steam_app", "wine64-preloader", "proton", "heroic", "lutris"];
    for proc_name in HEAVY_PROCESSES {
        if cmdline.contains(proc_name) {
            return true;
        }
    }
    false
}

fn check_if_fullscreen_game_active() -> bool {
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.filter_map(|e| e.ok()) {
            let is_pid = entry
                .file_name()
                .to_str()
                .map_or(false, |s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()));

            if is_pid {
                if let Ok(cmdline) = std::fs::read_to_string(entry.path().join("cmdline")) {
                    if is_heavy_process_cmdline(&cmdline) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_heavy_process_cmdline() {
        assert!(is_heavy_process_cmdline("/usr/bin/gamescope -W 1920"));
        assert!(is_heavy_process_cmdline("proton run game.exe"));
        assert!(is_heavy_process_cmdline("/opt/heroic/heroic"));
        assert!(is_heavy_process_cmdline("steam_app_12345"));
        assert!(!is_heavy_process_cmdline("/usr/bin/bash"));
        assert!(!is_heavy_process_cmdline("cosmic-panel"));
        assert!(!is_heavy_process_cmdline(""));
    }
}
