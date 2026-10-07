use crate::config::Config;
use crate::engine::{detect_outputs, MonitorOutput, WallpaperEngine};
use crate::scanner::{scan_directories, thumbs::generate_thumbnail, VideoItem};
use crate::theme::apply_cosmic_theme;
use crate::online::{
    download_online_thumbnail, download_to_file_with_progress, fetch_bing_archive_page, fetch_bing_wallpapers,
    fetch_motionbgs_wallpapers, fetch_wallhaven_wallpapers, fetch_minimalistic_wallpapers,
    OnlineSource, OnlineWallpaperItem,
};

use cosmic::app::Core;
use cosmic::iced::{Length, Subscription, Task};
use cosmic::iced::platform_specific::runtime::wayland::layer_surface::{IcedMargin, SctkLayerSurfaceSettings};
use cosmic::iced::platform_specific::shell::commands::layer_surface::{Anchor, KeyboardInteractivity, Layer};
use cosmic::surface::action::{app_layer_shell, destroy_layer_shell, LiveSettings};
use cosmic::widget::{self, nav_bar};
use cosmic::Element;
use std::path::PathBuf;
use std::time::{Duration, Instant};

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
    Favorites,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibrarySort {
    #[default]
    Newest,
    Oldest,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    SelectPage(nav_bar::Id),
    SelectLibraryFilter(LibraryFilter),
    LoadMoreLibraryWallpapers,
    ToggleFavorite(PathBuf),
    ToggleLibrarySort,
    SelectLibrarySort(LibrarySort),
    SetLanguage(crate::i18n::Language),
    SelectOutput(String),
    SelectHwdec(String),
    SelectGpuPreference(String),
    ToggleRotation(bool),
    ToggleRotationOnlyFavorites(bool),
    StartFavoritesRotation,
    NavigateToPage(Page),
    SelectInterval(u64),
    CustomIntervalChanged(String),
    ApplyCustomInterval,
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
    SelectThemeMode(crate::config::ThemeMode),
    ToggleMute(bool),
    ToggleSmartPause(bool),
    ToggleAutoPause(bool),
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
    SelectMotionbgsCategory(String),
    SelectMotionbgsResolution(String),
    MotionbgsSearchChanged(String),
    SubmitMotionbgsSearch,
    ClearMotionbgsSearch,
    SelectWallhavenCategory(String),
    SelectWallhavenSorting(String),
    SelectWallhavenResolution(String),
    WallhavenSearchChanged(String),
    SubmitWallhavenSearch,
    ClearWallhavenSearch,
    MinimalisticSearchChanged(String),
    ClearMinimalisticSearch,
    DownloadOnlineWallpaper { item: OnlineWallpaperItem, auto_apply: bool },
    CancelOnlineDownload(String),
    DownloadProgressUpdated { id: String, downloaded: u64, total: Option<u64>, percent: f32 },
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
    OpenQuickSwitcher,
    CloseQuickSwitcher,
    ToggleQuickSwitcher,
    SwitcherPrev,
    SwitcherNext,
    SwitcherPrevCol,
    SwitcherNextCol,
    SwitcherSelect(usize),
    SwitcherApply,
    SwitcherApplyIndex(usize),
    SwitcherNoop,
    SwitcherTogglePause,
    SwitcherToggleFavorite,
    SwitcherKeyPressed { window: cosmic::iced::window::Id, key: cosmic::iced::keyboard::Key },
    SwitcherUnfocused(cosmic::iced::window::Id),
    SwitcherWheelScrolled { window: cosmic::iced::window::Id, delta: cosmic::iced::mouse::ScrollDelta },
    SwitcherMouseClicked(cosmic::iced::window::Id),
    SwitcherCursorMoved { window: cosmic::iced::window::Id, position: cosmic::iced::Point },
    SwitcherCursorLeft(cosmic::iced::window::Id),
    SwitcherResized { window: cosmic::iced::window::Id, size: cosmic::iced::Size },
    SwitcherEdgeScrollTick,
    SwitcherCinematicHover(Option<i32>),
    SwitcherCinematicLeave(i32),
    SwitcherAnimTick,
    SelectTrayClickAction(String),
    ToggleSwitcherOnlyFavorites(bool),
    SelectSwitcherPosition(String),
    SelectSwitcherStyle(crate::config::SwitcherStyle),
    CopySwitcherCommand,
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
        cosmic::iced::Event::Window(cosmic::iced::window::Event::Unfocused) => {
            Some(Message::SwitcherUnfocused(window))
        }
        cosmic::iced::Event::Mouse(cosmic::iced::mouse::Event::ButtonPressed(cosmic::iced::mouse::Button::Left)) => {
            Some(Message::SwitcherMouseClicked(window))
        }
        cosmic::iced::Event::Mouse(cosmic::iced::mouse::Event::CursorMoved { position }) => {
            Some(Message::SwitcherCursorMoved { window, position })
        }
        cosmic::iced::Event::Mouse(cosmic::iced::mouse::Event::CursorLeft) => {
            Some(Message::SwitcherCursorLeft(window))
        }
        cosmic::iced::Event::Mouse(cosmic::iced::mouse::Event::WheelScrolled { delta }) => {
            Some(Message::SwitcherWheelScrolled { window, delta })
        }
        cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::KeyPressed {
            key,
            ..
        }) => {
            Some(Message::SwitcherKeyPressed { window, key })
        }
        cosmic::iced::Event::Window(cosmic::iced::window::Event::Resized(size)) => {
            Some(Message::SwitcherResized { window, size })
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
    pub(crate) motionbgs_wallpapers: Vec<OnlineWallpaperItem>,
    pub(crate) motionbgs_page: u32,
    pub(crate) motionbgs_category: String,
    pub(crate) motionbgs_resolution: String,
    pub(crate) motionbgs_search: String,
    pub(crate) bing_wallpapers: Vec<OnlineWallpaperItem>,
    pub(crate) wallhaven_wallpapers: Vec<OnlineWallpaperItem>,
    pub(crate) minimalistic_all_wallpapers: Vec<OnlineWallpaperItem>,
    pub(crate) minimalistic_wallpapers: Vec<OnlineWallpaperItem>,
    pub(crate) minimalistic_search: String,
    pub(crate) minimalistic_page: usize,
    pub(crate) explore_loading: bool,
    pub(crate) explore_loading_more: bool,
    pub(crate) explore_error: Option<String>,
    pub(crate) downloading_online_ids: std::collections::HashSet<String>,
    pub(crate) download_progress: std::collections::HashMap<String, (u64, Option<u64>, f32)>,
    pub(crate) download_cancels: std::collections::HashMap<String, std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub(crate) online_thumbs: std::collections::HashMap<String, PathBuf>,
    pub(crate) http_client: reqwest::Client,
    pub(crate) bing_page: u32,
    pub(crate) wallhaven_page: u32,
    pub(crate) wallhaven_category: String,
    pub(crate) wallhaven_sorting: String,
    pub(crate) wallhaven_resolution: String,
    pub(crate) wallhaven_search: String,
    pub(crate) library_filter: LibraryFilter,
    pub(crate) library_sort: LibrarySort,
    pub(crate) library_limit: usize,
    pub(crate) favorites_set: std::collections::HashSet<String>,
    pub(crate) category_counts: (usize, usize, usize, usize, usize),
    pub(crate) update_status: UpdateStatus,
    pub(crate) pending_auto_apply_id: Option<String>,
    pub(crate) last_update_check: Option<Instant>,
    pub(crate) switcher_window_id: Option<cosmic::iced::window::Id>,
    pub(crate) closing_switcher_window_id: Option<cosmic::iced::window::Id>,
    pub(crate) switcher_index: usize,
    pub(crate) custom_interval_input: String,
    pub(crate) switcher_scroll_accum: f32,
    pub(crate) switcher_last_scroll: Option<Instant>,
    pub(crate) switcher_last_event_time: Option<Instant>,
    pub(crate) switcher_window_width: f32,
    pub(crate) switcher_window_height: f32,
    pub(crate) switcher_cursor_position: Option<cosmic::iced::Point>,
    pub(crate) switcher_edge_scroll_dir: i8,
    pub(crate) switcher_last_edge_scroll: Option<Instant>,
    pub(crate) switcher_active_output: Option<String>,
    pub(crate) switcher_cinematic_hover: Option<i32>,
    pub(crate) switcher_cinematic_animating: bool,
    pub(crate) switcher_cinematic_scales: [f32; 7],
    pub(crate) switcher_cinematic_slide: f32,
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

    pub(crate) fn notify_applied(
        &mut self,
        text: impl Into<String>,
        action_label: impl Into<String>,
        action_msg: Message,
    ) -> Task<cosmic::Action<Message>> {
        // Clear previous toast notifications so they never stack up when switching wallpapers
        self.toasts = cosmic::widget::Toasts::new(Message::CloseToast);
        let toast = cosmic::widget::Toast::new(text)
            .duration(cosmic::widget::toaster::Duration::Short)
            .action(action_label.into(), move |_id| action_msg.clone());
        let task = self.toasts.push(toast);
        task.map(cosmic::Action::App)
    }

    pub(crate) fn set_status(&mut self, text: impl Into<String>) -> Task<cosmic::Action<Message>> {
        let msg = text.into();
        self.status_message = Some(msg.clone());
        self.notify(msg)
    }

    pub(crate) fn handle_topology_change(&mut self, current_outputs: Vec<MonitorOutput>) {
        if current_outputs == self.outputs {
            return;
        }

        let old_outputs = self.outputs.clone();
        let new_count = current_outputs.len();
        self.outputs = current_outputs.clone();

        // 1. If currently selected output was removed, fallback to first output or "*"
        if self.selected_output != "*" && !self.outputs.iter().any(|o| o.name == self.selected_output) {
            self.selected_output = self.outputs.first().map(|o| o.name.clone()).unwrap_or_else(|| "*".into());
        }

        // 2. Stop outputs that were disconnected
        let active = self.engine.processes_keys();
        for out in &active {
            if out != "*" && !self.outputs.iter().any(|o| o.name == *out) {
                self.engine.stop_output(out);
            }
        }

        // 3. Identify newly added outputs (e.g. plugged into docking station)
        let added_monitors: Vec<_> = self.outputs.iter()
            .filter(|new_o| !old_outputs.iter().any(|old_o| old_o.name == new_o.name))
            .cloned()
            .collect();

        // 4. If wildcard "*" was active, restart it so mpvpaper binds to the new screens
        if active.contains(&"*".to_string()) {
            if let Some(curr) = &self.config.current {
                let sc = self.config.scaling.get("*").cloned().unwrap_or_else(|| "fit".into());
                let _ = self.engine.set_wallpaper(
                    "*",
                    curr,
                    &sc,
                    self.config.mute,
                    self.config.volume,
                    &self.config.hwdec,
                    self.config.auto_pause,
                    &self.config.gpu_preference,
                );
            }
        } else if !added_monitors.is_empty() {
            // Apply wallpaper to newly connected outputs
            for monitor in added_monitors {
                let path_opt = self.config.wallpapers.get(&monitor.name)
                    .cloned()
                    .or_else(|| self.config.wallpapers.get("*").cloned())
                    .or_else(|| self.config.current.clone());

                if let Some(path) = path_opt {
                    if std::path::Path::new(&path).exists() {
                        let sc = self.config.scaling.get(&monitor.name)
                            .cloned()
                            .unwrap_or_else(|| "fit".into());
                        let _ = self.engine.set_wallpaper(
                            &monitor.name,
                            &path,
                            &sc,
                            self.config.mute,
                            self.config.volume,
                            &self.config.hwdec,
                            self.config.auto_pause,
                            &self.config.gpu_preference,
                        );
                    }
                }
            }
        }

        self.status_message = Some(self.language.status_topology_updated(new_count));
        self.status_timer = 5;
    }

    pub(crate) fn apply_minimalistic_filter(&mut self) -> Vec<OnlineWallpaperItem> {
        let query = self.minimalistic_search.trim().to_lowercase();
        let filtered: Vec<OnlineWallpaperItem> = if query.is_empty() {
            self.minimalistic_all_wallpapers.clone()
        } else {
            self.minimalistic_all_wallpapers
                .iter()
                .filter(|item| {
                    item.title.to_lowercase().contains(&query)
                        || item.author_or_copyright.to_lowercase().contains(&query)
                })
                .cloned()
                .collect()
        };

        let count_to_take = (self.minimalistic_page * 24).max(24);
        let paged: Vec<OnlineWallpaperItem> = filtered.into_iter().take(count_to_take).collect();
        self.minimalistic_wallpapers = paged.clone();
        paged
    }

    pub(crate) fn queue_thumbnails(&mut self, items: &[OnlineWallpaperItem]) -> Task<cosmic::Action<Message>> {
        let mut thumb_tasks = Vec::new();
        let client = self.http_client.clone();
        for item in items {
            let thumb_dest = item.local_thumb_path();
            let is_compact = if let Ok(m) = thumb_dest.metadata() {
                m.is_file() && m.len() > 500 && m.len() <= 400_000
            } else {
                false
            };

            if is_compact {
                self.online_thumbs.insert(item.id.clone(), thumb_dest);
            } else {
                let c = client.clone();
                let id = item.id.clone();
                let url = item.thumb_url.clone();
                let full = item.full_url.clone();
                thumb_tasks.push(Task::perform(
                    async move {
                        if let Ok(path) = download_online_thumbnail(&c, &url, &full, &thumb_dest).await {
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
            Task::batch(thumb_tasks)
        } else {
            Task::none()
        }
    }

    pub(crate) fn update_category_counts(&mut self) {
        let mut count_all = 0;
        let mut count_live = 0;
        let mut count_static = 0;
        let mut count_downloaded = 0;
        let mut count_favorites = 0;

        for v in &self.videos {
            count_all += 1;
            if v.is_video {
                count_live += 1;
            } else {
                count_static += 1;
            }
            if v.is_downloaded {
                count_downloaded += 1;
            }
            if self.favorites_set.contains(v.path.to_string_lossy().as_ref()) {
                count_favorites += 1;
            }
        }

        self.category_counts = (count_all, count_live, count_static, count_downloaded, count_favorites);
    }

    pub(crate) fn sort_videos(&mut self) {
        match self.library_sort {
            LibrarySort::Newest => {
                self.videos.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.name_lower.cmp(&b.name_lower)));
            }
            LibrarySort::Oldest => {
                self.videos.sort_by(|a, b| a.modified.cmp(&b.modified).then_with(|| a.name_lower.cmp(&b.name_lower)));
            }
        }
    }

    pub(crate) fn rescan_and_update(&mut self) {
        self.videos = scan_directories(&self.config.dirs, &self.config.custom_videos);
        self.sort_videos();
        self.update_category_counts();
    }

    pub(crate) fn rotation_pool(&self) -> Vec<PathBuf> {
        if self.config.rotation_only_favorites {
            let favs: Vec<PathBuf> = self.videos.iter()
                .filter(|v| self.favorites_set.contains(v.path.to_string_lossy().as_ref()))
                .map(|v| v.path.clone())
                .collect();
            if !favs.is_empty() {
                return favs;
            }
        }
        self.videos.iter().map(|v| v.path.clone()).collect()
    }

    pub(crate) fn switcher_pool(&self) -> Vec<&VideoItem> {
        if self.config.switcher_only_favorites {
            let favs: Vec<&VideoItem> = self.videos.iter()
                .filter(|v| self.favorites_set.contains(v.path.to_string_lossy().as_ref()))
                .collect();
            if !favs.is_empty() {
                return favs;
            }
        }
        self.videos.iter().collect()
    }

    pub(crate) fn prewarm_switcher_around(&self, curr_idx: usize) {
        let pool = self.switcher_pool();
        let n = pool.len();
        if n == 0 {
            return;
        }
        let accent_base = cosmic::theme::active().cosmic().accent.base;
        let accent_color: cosmic::iced::Color = accent_base.into();
        let accent_rgb: [u8; 3] = [
            (accent_color.r * 255.0).round() as u8,
            (accent_color.g * 255.0).round() as u8,
            (accent_color.b * 255.0).round() as u8,
        ];

        match self.config.switcher_style {
            crate::config::SwitcherStyle::Cinematic => {
                let mut targets = Vec::new();
                for offset in -4..=4 {
                    let idx = ((curr_idx as i32 + offset).rem_euclid(n as i32)) as usize;
                    targets.push(pool[idx].path.clone());
                }
                crate::scanner::thumbs::prewarm_cinematic_thumbs(&targets, accent_rgb);
            }
            crate::config::SwitcherStyle::Honeycomb => {
                let mut targets = Vec::new();
                for offset in -12..=12 {
                    let idx = ((curr_idx as i32 + offset).rem_euclid(n as i32)) as usize;
                    targets.push(pool[idx].path.clone());
                }
                crate::scanner::thumbs::prewarm_honeycomb_thumbs(&targets, accent_rgb);
            }
            crate::config::SwitcherStyle::Classic => {}
        }
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

        // Cleanup any stale temporary download artifacts (.tmp / .tmp.jpg)
        let online_dir = crate::online::wallpapers_online_dir();
        if let Ok(entries) = std::fs::read_dir(&online_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let p = entry.path();
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    if name.ends_with(".tmp") || name.ends_with(".tmp.jpg") {
                        let _ = std::fs::remove_file(p);
                    }
                }
            }
        }

        // Cleanup any oversized legacy thumbnails (> 350KB) so they are regenerated compactly
        let cache_dir = crate::scanner::thumbs::get_cache_dir();
        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() && meta.len() > 350_000 {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
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
                    let _ = engine.set_wallpaper(out, path, &sc, config.mute, config.volume, &config.hwdec, config.auto_pause, &config.gpu_preference);
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
                "switcher" => tasks.push(Task::done(cosmic::Action::App(Message::ToggleQuickSwitcher))),
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
        let library_sort = match config.library_sort.as_str() {
            "oldest" => LibrarySort::Oldest,
            _ => LibrarySort::Newest,
        };

        let initial_interval = config.interval;
        let favorites_set: std::collections::HashSet<String> = config.favorites.iter().cloned().collect();

        let mut app = Self {
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
            explore_source: OnlineSource::MotionBGS,
            motionbgs_wallpapers: Vec::new(),
            motionbgs_page: 1,
            motionbgs_category: "all".into(),
            motionbgs_resolution: "4k".into(),
            motionbgs_search: String::new(),
            bing_wallpapers: Vec::new(),
            wallhaven_wallpapers: Vec::new(),
            minimalistic_all_wallpapers: Vec::new(),
            minimalistic_wallpapers: Vec::new(),
            minimalistic_search: String::new(),
            minimalistic_page: 1,
            explore_loading: false,
            explore_loading_more: false,
            explore_error: None,
            downloading_online_ids: std::collections::HashSet::new(),
            download_progress: std::collections::HashMap::new(),
            download_cancels: std::collections::HashMap::new(),
            online_thumbs: std::collections::HashMap::new(),
            http_client: reqwest::Client::new(),
            bing_page: 1,
            wallhaven_page: 1,
            wallhaven_category: "110".into(),
            wallhaven_sorting: "toplist".into(),
            wallhaven_resolution: "all".into(),
            wallhaven_search: String::new(),
            library_filter: LibraryFilter::All,
            library_sort,
            library_limit: 48,
            favorites_set,
            category_counts: (0, 0, 0, 0, 0),
            update_status: UpdateStatus::Idle,
            pending_auto_apply_id: None,
            last_update_check: None,
            switcher_window_id: None,
            closing_switcher_window_id: None,
            switcher_index: 0,
            custom_interval_input: initial_interval.to_string(),
            switcher_scroll_accum: 0.0,
            switcher_last_scroll: None,
            switcher_last_event_time: None,
            switcher_window_width: 1920.0,
            switcher_window_height: 1080.0,
            switcher_cursor_position: None,
            switcher_edge_scroll_dir: 0,
            switcher_last_edge_scroll: None,
            switcher_active_output: None,
            switcher_cinematic_hover: None,
            switcher_cinematic_animating: false,
            switcher_cinematic_scales: [0.0; 7],
            switcher_cinematic_slide: 0.0,
        };

        app.sort_videos();
        app.update_category_counts();

        if !crate::online::updater::is_flatpak() {
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
                    OnlineSource::MotionBGS => self.motionbgs_wallpapers.is_empty(),
                    OnlineSource::Bing => self.bing_wallpapers.is_empty(),
                    OnlineSource::Wallhaven => self.wallhaven_wallpapers.is_empty(),
                    OnlineSource::Minimalistic => self.minimalistic_all_wallpapers.is_empty(),
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
                    "switcher" => Task::done(cosmic::Action::App(Message::ToggleQuickSwitcher)),
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

    fn view_window(&self, id: cosmic::iced::window::Id) -> Element<'_, Self::Message> {
        if self.switcher_window_id == Some(id) {
            self.view_quick_switcher()
        } else if self.core().main_window_id() == Some(id) {
            self.view()
        } else {
            widget::container(widget::Space::new())
                .width(cosmic::iced::Length::Fill)
                .height(cosmic::iced::Length::Fill)
                .class(cosmic::theme::Container::Transparent)
                .into()
        }
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        if self.switcher_window_id.is_some() || self.closing_switcher_window_id.is_some() {
            let theme = cosmic::theme::active();
            return Some(cosmic::iced::theme::Style {
                background_color: cosmic::iced::Color::TRANSPARENT,
                icon_color: theme.cosmic().on_bg_color().into(),
                text_color: theme.cosmic().on_bg_color().into(),
            });
        }
        None
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

        if self.switcher_window_id.is_some() && self.switcher_edge_scroll_dir != 0 {
            subs.push(
                cosmic::iced::time::every(Duration::from_millis(180))
                    .map(|_| Message::SwitcherEdgeScrollTick)
            );
        }

        let cinematic_animating = self.switcher_cinematic_animating || self.switcher_cinematic_slide.abs() > 0.5;
        if self.switcher_window_id.is_some()
            && self.config.switcher_style == crate::config::SwitcherStyle::Cinematic
            && cinematic_animating
        {
            subs.push(
                cosmic::iced::time::every(Duration::from_millis(16))
                    .map(|_| Message::SwitcherAnimTick)
            );
        }

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
            cosmic::iced::time::every(Duration::from_secs(3))
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

        if !crate::online::updater::is_flatpak() {
            subs.push(
                cosmic::iced::time::every(Duration::from_secs(4 * 3600))
                    .map(|_| Message::CheckForUpdates { user_initiated: false })
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
                            OnlineSource::MotionBGS => self.motionbgs_wallpapers.is_empty(),
                            OnlineSource::Bing => self.bing_wallpapers.is_empty(),
                            OnlineSource::Wallhaven => self.wallhaven_wallpapers.is_empty(),
                            OnlineSource::Minimalistic => self.minimalistic_all_wallpapers.is_empty(),
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
                return self.set_status(self.language.status_output_selected(&out));
            }

            Message::SelectHwdec(hwdec) => {
                self.config.hwdec = hwdec.clone();
                let _ = self.config.save();
                for (output, path) in self.config.wallpapers.clone() {
                    let sc = self.config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());
                    let _ = self.engine.set_wallpaper(&output, &path, &sc, self.config.mute, self.config.volume, &self.config.hwdec, self.config.auto_pause, &self.config.gpu_preference);
                }
                return self.set_status(self.language.status_hwdec_selected(&hwdec));
            }

            Message::SelectGpuPreference(gpu_pref) => {
                self.config.gpu_preference = gpu_pref.clone();
                let _ = self.config.save();
                for (output, path) in self.config.wallpapers.clone() {
                    let sc = self.config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());
                    let _ = self.engine.set_wallpaper(&output, &path, &sc, self.config.mute, self.config.volume, &self.config.hwdec, self.config.auto_pause, &self.config.gpu_preference);
                }
                return self.set_status(self.language.status_gpu_selected(&gpu_pref));
            }

            Message::ToggleRotation(active) => {
                self.config.rotation = active;
                let _ = self.config.save();
                let status_msg = if active { self.language.status_rotation_enabled() } else { self.language.status_rotation_disabled() };
                return self.set_status(status_msg);
            }

            Message::ToggleRotationOnlyFavorites(active) => {
                self.config.rotation_only_favorites = active;
                let _ = self.config.save();
                let status_msg = if active {
                    self.language.status_rotation_source_favs()
                } else {
                    self.language.status_rotation_source_all()
                };
                return self.set_status(status_msg);
            }

            Message::StartFavoritesRotation => {
                self.config.rotation = true;
                self.config.rotation_only_favorites = true;
                if self.config.interval == 0 {
                    self.config.interval = 15;
                }
                let _ = self.config.save();

                let pool = self.rotation_pool();
                let mut tasks = Vec::new();
                if let Some(first_path) = pool.first() {
                    let path_str = first_path.to_string_lossy();
                    let current_is_fav = self.config.current.as_deref().map(|c| c == path_str).unwrap_or(false);
                    if !current_is_fav {
                        let output = self.selected_output.clone();
                        tasks.push(Task::done(cosmic::Action::App(Message::ApplyWallpaper {
                            video_path: first_path.clone(),
                            output,
                        })));
                    }
                }
                let status_msg = self.language.status_favs_rotation_started(self.config.interval);
                tasks.push(self.set_status(status_msg));
                return Task::batch(tasks);
            }

            Message::NavigateToPage(page) => {
                let pos = match page {
                    Page::Library => 0,
                    Page::Explore => 1,
                    Page::Monitors => 2,
                    Page::Settings => 3,
                    Page::About => 4,
                };
                self.nav.activate_position(pos);
                self.active_page = page;
                return Task::none();
            }

            Message::SelectInterval(interval) => {
                self.config.interval = interval;
                self.custom_interval_input = interval.to_string();
                let _ = self.config.save();
                return self.set_status(self.language.status_interval_selected(interval));
            }

            Message::CustomIntervalChanged(text) => {
                let digits: String = text.chars().filter(|c| c.is_ascii_digit()).take(5).collect();
                self.custom_interval_input = digits;
                return Task::none();
            }

            Message::ApplyCustomInterval => {
                if let Ok(val) = self.custom_interval_input.trim().parse::<u64>() {
                    if (1..=1440).contains(&val) {
                        self.config.interval = val;
                        self.custom_interval_input = val.to_string();
                        let _ = self.config.save();
                        return self.set_status(self.language.status_interval_selected(val));
                    }
                }
                self.custom_interval_input = self.config.interval.to_string();
                let warn_msg = self.language.status_interval_invalid().to_string();
                return self.notify(warn_msg);
            }

            Message::SelectRotationOrder(order) => {
                self.config.order = order.clone();
                let _ = self.config.save();
                return self.set_status(self.language.status_rotation_order_selected(&order));
            }

            Message::TogglePauseOnBattery(active) => {
                self.config.pause_on_battery = active;
                let _ = self.config.save();
                return self.set_status(if active { self.language.status_pause_battery_enabled() } else { self.language.status_pause_battery_disabled() });
            }

            Message::ChangeVolume(vol) => {
                self.config.volume = vol;
                let _ = self.config.save();
                if !self.config.mute {
                    self.engine.set_volume(None, vol);
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
                let dead = self.engine.reap_dead_processes();
                if !dead.is_empty() && self.engine.processes_keys().is_empty() {
                    self.is_paused = false;
                    self.tray_controller.update_state(String::new(), false, false);
                }

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
                crate::tray::TrayAction::Activate => {
                    if self.config.tray_click_action == "main_window" {
                        return Task::done(cosmic::Action::App(Message::ShowMainWindow));
                    } else {
                        return Task::done(cosmic::Action::App(Message::ToggleQuickSwitcher));
                    }
                }
                crate::tray::TrayAction::OpenSwitcher => {
                    return Task::done(cosmic::Action::App(Message::OpenQuickSwitcher));
                }
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
                let mut tasks = Vec::new();

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
                    tasks.push(open_task.discard());
                    tasks.push(cosmic::iced::window::gain_focus(new_id));
                } else if let Some(id) = self.core().main_window_id() {
                    tasks.push(cosmic::iced::window::minimize(id, false));
                    tasks.push(cosmic::iced::window::gain_focus(id));
                }

                if !crate::online::updater::is_flatpak() {
                    if let UpdateStatus::Available { ref latest_tag, ref download_url, .. } = self.update_status {
                        let toast_text = format!("🚀 {} {}", self.language.about_update_available(), latest_tag);
                        if let Some(url) = download_url {
                            let action_lbl = self.language.toast_update_available_action();
                            tasks.push(self.notify_with_action(
                                toast_text,
                                action_lbl,
                                Message::PerformGuiUpdate { download_url: url.clone(), version: latest_tag.clone() },
                            ));
                        } else {
                            tasks.push(self.notify_with_action(
                                toast_text,
                                self.language.about_github_btn(),
                                Message::OpenGitHub,
                            ));
                        }
                    } else {
                        let should_check = match self.last_update_check {
                            Some(last) => last.elapsed() > Duration::from_secs(45 * 60),
                            None => true,
                        };
                        if should_check && !matches!(self.update_status, UpdateStatus::Checking | UpdateStatus::Downloading) {
                            tasks.push(Task::done(cosmic::Action::App(Message::CheckForUpdates { user_initiated: false })));
                        }
                    }
                }

                return Task::batch(tasks);
            }

            Message::OpenQuickSwitcher => {
                let pool = self.switcher_pool();
                let curr_idx = if let Some(curr) = &self.config.current {
                    pool.iter()
                        .position(|v| v.path.to_string_lossy() == *curr)
                        .unwrap_or(0)
                } else {
                    0
                };
                self.switcher_index = curr_idx;
                self.switcher_scroll_accum = 0.0;
                self.switcher_last_scroll = None;
                self.switcher_last_event_time = None;
                let (def_w, def_h) = self.outputs.iter()
                    .find(|o| o.name == self.selected_output)
                    .map(|o| (o.width as f32, o.height as f32))
                    .unwrap_or_else(|| {
                        self.outputs.first()
                            .map(|o| (o.width as f32, o.height as f32))
                            .unwrap_or((1920.0, 1080.0))
                    });
                self.switcher_window_width = def_w;
                self.switcher_window_height = def_h;
                self.switcher_cursor_position = None;
                self.switcher_edge_scroll_dir = 0;
                self.switcher_last_edge_scroll = None;
                self.switcher_active_output = None;
                self.switcher_cinematic_hover = None;
                self.switcher_cinematic_animating = false;
                self.switcher_cinematic_scales = [0.0; 7];
                self.switcher_cinematic_slide = 0.0;
                self.prewarm_switcher_around(curr_idx);

                if let Some(id) = self.switcher_window_id {
                    return cosmic::iced::window::gain_focus(id);
                }

                self.closing_switcher_window_id = None;

                let id = cosmic::iced::window::Id::unique();
                self.switcher_window_id = Some(id);

                let is_fullscreen_overlay = matches!(self.config.switcher_style, crate::config::SwitcherStyle::Cinematic | crate::config::SwitcherStyle::Honeycomb);
                let surface_action = app_layer_shell(
                    move |_app: &AuraApp| {
                        LiveSettings {
                            blur: Some(is_fullscreen_overlay),
                            ..Default::default()
                        }
                    },
                    move |_app: &mut AuraApp| {
                        let mut settings = SctkLayerSurfaceSettings {
                            id,
                            layer: Layer::Overlay,
                            keyboard_interactivity: KeyboardInteractivity::Exclusive,
                            anchor: Anchor::TOP.union(Anchor::BOTTOM).union(Anchor::LEFT).union(Anchor::RIGHT),
                            margin: IcedMargin::default(),
                            size: None,
                            namespace: "aura-switcher".to_string(),
                            ..Default::default()
                        };
                        if is_fullscreen_overlay {
                            settings.exclusive_zone = -1;
                            settings.size_limits = cosmic::iced::Limits::NONE
                                .min_width(1.0)
                                .min_height(1.0)
                                .max_width(f32::INFINITY)
                                .max_height(f32::INFINITY);
                        }
                        settings
                    },
                    None,
                );

                return Task::done(cosmic::Action::Cosmic(cosmic::app::Action::Surface(surface_action)));
            }

            Message::CloseQuickSwitcher => {
                self.switcher_scroll_accum = 0.0;
                self.switcher_last_scroll = None;
                self.switcher_last_event_time = None;
                self.switcher_cursor_position = None;
                self.switcher_edge_scroll_dir = 0;
                self.switcher_last_edge_scroll = None;
                self.switcher_active_output = None;
                self.switcher_cinematic_hover = None;
                self.switcher_cinematic_animating = false;
                self.switcher_cinematic_scales = [0.0; 7];
                self.switcher_cinematic_slide = 0.0;
                if let Some(id) = self.switcher_window_id.take() {
                    self.closing_switcher_window_id = Some(id);
                    return Task::done(cosmic::Action::Cosmic(cosmic::app::Action::Surface(destroy_layer_shell(id))));
                }
            }

            Message::ToggleQuickSwitcher => {
                if self.switcher_window_id.is_some() {
                    return Task::done(cosmic::Action::App(Message::CloseQuickSwitcher));
                } else {
                    return Task::done(cosmic::Action::App(Message::OpenQuickSwitcher));
                }
            }

            Message::SwitcherPrev => {
                let n = self.switcher_pool().len();
                if n > 0 {
                    self.switcher_index = (self.switcher_index + n - 1) % n;
                    self.switcher_cinematic_hover = None;
                    self.switcher_cinematic_animating = true;
                    self.switcher_cinematic_scales = [0.0; 7];
                    self.switcher_cinematic_slide = (self.switcher_cinematic_slide - 200.0).clamp(-400.0, 400.0);
                    self.prewarm_switcher_around(self.switcher_index);
                }
            }

            Message::SwitcherNext => {
                let n = self.switcher_pool().len();
                if n > 0 {
                    self.switcher_index = (self.switcher_index + 1) % n;
                    self.switcher_cinematic_hover = None;
                    self.switcher_cinematic_animating = true;
                    self.switcher_cinematic_scales = [0.0; 7];
                    self.switcher_cinematic_slide = (self.switcher_cinematic_slide + 200.0).clamp(-400.0, 400.0);
                    self.prewarm_switcher_around(self.switcher_index);
                }
            }

            Message::SwitcherPrevCol => {
                let n = self.switcher_pool().len();
                if n > 0 {
                    let rows = crate::ui::switcher::HoneycombLayout::ROWS_PER_COL;
                    let step = rows.min(n);
                    self.switcher_index = self.switcher_index.saturating_sub(step);
                    self.prewarm_switcher_around(self.switcher_index);
                }
            }

            Message::SwitcherNextCol => {
                let n = self.switcher_pool().len();
                if n > 0 {
                    let rows = crate::ui::switcher::HoneycombLayout::ROWS_PER_COL;
                    let step = rows.min(n);
                    self.switcher_index = (self.switcher_index + step).min(n - 1);
                    self.prewarm_switcher_around(self.switcher_index);
                }
            }

            Message::SwitcherSelect(idx) => {
                let pool = self.switcher_pool();
                if idx < pool.len() {
                    self.switcher_index = idx;
                    self.prewarm_switcher_around(self.switcher_index);
                }
            }

            Message::SwitcherApplyIndex(idx) => {
                let mut tasks = Vec::new();
                let pool = self.switcher_pool();
                if let Some(video) = pool.get(idx) {
                    let target_output = self.switcher_active_output.as_ref().unwrap_or(&self.selected_output).clone();
                    tasks.push(Task::done(cosmic::Action::App(Message::ApplyWallpaper {
                        video_path: video.path.clone(),
                        output: target_output,
                    })));
                }
                if let Some(id) = self.switcher_window_id.take() {
                    self.closing_switcher_window_id = Some(id);
                    self.switcher_active_output = None;
                    tasks.push(Task::done(cosmic::Action::Cosmic(cosmic::app::Action::Surface(destroy_layer_shell(id)))));
                }
                return Task::batch(tasks);
            }

            Message::SwitcherApply => {
                return Task::done(cosmic::Action::App(Message::SwitcherApplyIndex(self.switcher_index)));
            }

            Message::SwitcherNoop => {
                return Task::none();
            }

            Message::SwitcherTogglePause => {
                let now_paused = self.engine.toggle_pause();
                self.is_paused = now_paused;
                let current_title = self.config.current.as_ref()
                    .and_then(|c| std::path::Path::new(c).file_stem().map(|s| s.to_string_lossy().to_string()))
                    .unwrap_or_else(|| "Video".into());
                self.tray_controller.update_state(current_title, now_paused, self.config.current.is_some());
            }

            Message::SwitcherToggleFavorite => {
                let pool = self.switcher_pool();
                if let Some(video) = pool.get(self.switcher_index) {
                    let path = video.path.clone();
                    return Task::done(cosmic::Action::App(Message::ToggleFavorite(path)));
                }
            }

            Message::SwitcherKeyPressed { window, key } => {
                if self.switcher_window_id == Some(window) {
                    let is_honeycomb = self.config.switcher_style == crate::config::SwitcherStyle::Honeycomb;
                    match key {
                        cosmic::iced::keyboard::Key::Named(cosmic::iced::keyboard::key::Named::ArrowLeft) => {
                            if is_honeycomb {
                                return Task::done(cosmic::Action::App(Message::SwitcherPrevCol));
                            } else {
                                return Task::done(cosmic::Action::App(Message::SwitcherPrev));
                            }
                        }
                        cosmic::iced::keyboard::Key::Named(cosmic::iced::keyboard::key::Named::ArrowRight) => {
                            if is_honeycomb {
                                return Task::done(cosmic::Action::App(Message::SwitcherNextCol));
                            } else {
                                return Task::done(cosmic::Action::App(Message::SwitcherNext));
                            }
                        }
                        cosmic::iced::keyboard::Key::Named(cosmic::iced::keyboard::key::Named::ArrowUp) => {
                            return Task::done(cosmic::Action::App(Message::SwitcherPrev));
                        }
                        cosmic::iced::keyboard::Key::Named(cosmic::iced::keyboard::key::Named::ArrowDown) => {
                            return Task::done(cosmic::Action::App(Message::SwitcherNext));
                        }
                        cosmic::iced::keyboard::Key::Named(cosmic::iced::keyboard::key::Named::Enter) => {
                            return Task::done(cosmic::Action::App(Message::SwitcherApply));
                        }
                        cosmic::iced::keyboard::Key::Named(cosmic::iced::keyboard::key::Named::Escape) => {
                            return Task::done(cosmic::Action::App(Message::CloseQuickSwitcher));
                        }
                        cosmic::iced::keyboard::Key::Character(ref c) if c == " " => {
                            return Task::done(cosmic::Action::App(Message::SwitcherTogglePause));
                        }
                        cosmic::iced::keyboard::Key::Character(ref c) if c.eq_ignore_ascii_case("f") => {
                            return Task::done(cosmic::Action::App(Message::SwitcherToggleFavorite));
                        }
                        cosmic::iced::keyboard::Key::Character(ref c) if c.eq_ignore_ascii_case("a") => {
                            if is_honeycomb {
                                return Task::done(cosmic::Action::App(Message::SwitcherPrevCol));
                            } else {
                                return Task::done(cosmic::Action::App(Message::SwitcherPrev));
                            }
                        }
                        cosmic::iced::keyboard::Key::Character(ref c) if c.eq_ignore_ascii_case("d") => {
                            if is_honeycomb {
                                return Task::done(cosmic::Action::App(Message::SwitcherNextCol));
                            } else {
                                return Task::done(cosmic::Action::App(Message::SwitcherNext));
                            }
                        }
                        cosmic::iced::keyboard::Key::Character(ref c) if c.eq_ignore_ascii_case("w") => {
                            return Task::done(cosmic::Action::App(Message::SwitcherPrev));
                        }
                        cosmic::iced::keyboard::Key::Character(ref c) if c.eq_ignore_ascii_case("s") => {
                            return Task::done(cosmic::Action::App(Message::SwitcherNext));
                        }
                        _ => {}
                    }
                }
            }

            Message::SwitcherUnfocused(id) => {
                if self.switcher_window_id == Some(id) {
                    return Task::done(cosmic::Action::App(Message::CloseQuickSwitcher));
                }
            }

            Message::SwitcherWheelScrolled { window, delta } => {
                if self.switcher_window_id == Some(window) {
                    const TOUCHPAD_THRESHOLD: f32 = 60.0;
                    const SCROLL_COOLDOWN: std::time::Duration = std::time::Duration::from_millis(130);
                    const GESTURE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(220);

                    let now = Instant::now();
                    if let Some(last_event) = self.switcher_last_event_time {
                        if now.duration_since(last_event) > GESTURE_TIMEOUT {
                            self.switcher_scroll_accum = 0.0;
                        }
                    }
                    self.switcher_last_event_time = Some(now);

                    let is_vertical = matches!(self.config.switcher_position.as_str(), "left" | "right");

                    let increment = match delta {
                        cosmic::iced::mouse::ScrollDelta::Lines { x, y } => {
                            let dir = if is_vertical {
                                if y.abs() >= x.abs() { -y } else { x }
                            } else {
                                if x.abs() >= y.abs() { x } else { -y }
                            };
                            dir * TOUCHPAD_THRESHOLD
                        }
                        cosmic::iced::mouse::ScrollDelta::Pixels { x, y } => {
                            if is_vertical {
                                if y.abs() >= x.abs() { -y } else { x }
                            } else {
                                if x.abs() >= y.abs() { x } else { -y }
                            }
                        }
                    };

                    // If direction reversed, reset accumulator to avoid fighting previous inertia
                    if (increment > 0.0 && self.switcher_scroll_accum < 0.0)
                        || (increment < 0.0 && self.switcher_scroll_accum > 0.0)
                    {
                        self.switcher_scroll_accum = 0.0;
                    }

                    self.switcher_scroll_accum += increment;

                    let can_step = match self.switcher_last_scroll {
                        Some(last) => now.duration_since(last) >= SCROLL_COOLDOWN,
                        None => true,
                    };

                    if can_step {
                        if self.switcher_scroll_accum >= TOUCHPAD_THRESHOLD {
                            self.switcher_scroll_accum = 0.0;
                            self.switcher_last_scroll = Some(now);
                            if self.config.switcher_style == crate::config::SwitcherStyle::Honeycomb {
                                return Task::done(cosmic::Action::App(Message::SwitcherNextCol));
                            } else {
                                return Task::done(cosmic::Action::App(Message::SwitcherNext));
                            }
                        } else if self.switcher_scroll_accum <= -TOUCHPAD_THRESHOLD {
                            self.switcher_scroll_accum = 0.0;
                            self.switcher_last_scroll = Some(now);
                            if self.config.switcher_style == crate::config::SwitcherStyle::Honeycomb {
                                return Task::done(cosmic::Action::App(Message::SwitcherPrevCol));
                            } else {
                                return Task::done(cosmic::Action::App(Message::SwitcherPrev));
                            }
                        }
                    }
                }
            }

            Message::SwitcherMouseClicked(window) => {
                if self.switcher_window_id == Some(window) && self.config.switcher_style == crate::config::SwitcherStyle::Honeycomb {
                    let pool = self.switcher_pool();
                    let n = pool.len();
                    if let Some(pos) = self.switcher_cursor_position {
                        let stored_w = f32::from_bits(crate::ui::switcher::SWITCHER_LOGICAL_WIDTH.load(std::sync::atomic::Ordering::Relaxed));
                        let stored_h = f32::from_bits(crate::ui::switcher::SWITCHER_LOGICAL_HEIGHT.load(std::sync::atomic::Ordering::Relaxed));
                        let width = if stored_w > 100.0 { stored_w } else if self.switcher_window_width > 100.0 { self.switcher_window_width } else { 1366.0 };
                        let height = if stored_h > 100.0 { stored_h } else if self.switcher_window_height > 100.0 { self.switcher_window_height } else { 768.0 };
                        let layout = crate::ui::switcher::HoneycombLayout::new(width, height, n, self.switcher_index);
                        if let Some(idx) = layout.hit_test(n, pos.x, pos.y) {
                            return Task::done(cosmic::Action::App(Message::SwitcherApplyIndex(idx)));
                        } else {
                            return Task::done(cosmic::Action::App(Message::CloseQuickSwitcher));
                        }
                    } else {
                        return Task::done(cosmic::Action::App(Message::CloseQuickSwitcher));
                    }
                }
            }

            Message::SwitcherCursorMoved { window, position } => {
                if self.switcher_window_id == Some(window) {
                    self.switcher_cursor_position = Some(position);
                    if self.config.switcher_style == crate::config::SwitcherStyle::Cinematic {
                        let pool_len = self.switcher_pool().len();
                        let new_hover = crate::ui::switcher::cinematic_hit_test(
                            self.switcher_window_width,
                            self.switcher_window_height,
                            position.x,
                            position.y,
                            self.switcher_cinematic_slide,
                            &self.switcher_cinematic_scales,
                            pool_len,
                        );
                        if self.switcher_cinematic_hover != new_hover {
                            self.switcher_cinematic_hover = new_hover;
                            self.switcher_cinematic_animating = true;
                        }
                    } else if self.config.switcher_style == crate::config::SwitcherStyle::Honeycomb {
                        let stored_w = f32::from_bits(crate::ui::switcher::SWITCHER_LOGICAL_WIDTH.load(std::sync::atomic::Ordering::Relaxed));
                        let width = if stored_w > 100.0 { stored_w } else if self.switcher_window_width > 100.0 { self.switcher_window_width } else { 1366.0 };
                        let edge_margin = (width * 0.08).clamp(70.0, 130.0);
                        let prev_dir = self.switcher_edge_scroll_dir;
                        let new_dir = if position.x < edge_margin {
                            -1
                        } else if position.x > width - edge_margin {
                            1
                        } else {
                            0
                        };
                        self.switcher_edge_scroll_dir = new_dir;

                        if new_dir != 0 && prev_dir == 0 {
                            let now = Instant::now();
                            let should_step = self.switcher_last_edge_scroll
                                .map(|last| now.duration_since(last) > Duration::from_millis(180))
                                .unwrap_or(true);
                            if should_step {
                                let pool_len = self.switcher_pool().len();
                                let rows = crate::ui::switcher::HoneycombLayout::ROWS_PER_COL;
                                let total_cols = (pool_len + rows - 1) / rows;
                                let sel_col = self.switcher_index / rows;
                                if new_dir > 0 && sel_col + 1 < total_cols {
                                    self.switcher_last_edge_scroll = Some(now);
                                    return Task::done(cosmic::Action::App(Message::SwitcherNextCol));
                                } else if new_dir < 0 && sel_col > 0 {
                                    self.switcher_last_edge_scroll = Some(now);
                                    return Task::done(cosmic::Action::App(Message::SwitcherPrevCol));
                                }
                            }
                        }
                    }
                }
            }

            Message::SwitcherCursorLeft(window) => {
                if self.switcher_window_id == Some(window) {
                    self.switcher_cursor_position = None;
                    self.switcher_edge_scroll_dir = 0;
                    if self.switcher_cinematic_hover.is_some() {
                        self.switcher_cinematic_hover = None;
                        self.switcher_cinematic_animating = true;
                    }
                }
            }

            Message::SwitcherCinematicHover(offset) => {
                if self.switcher_cinematic_hover != offset {
                    self.switcher_cinematic_hover = offset;
                    self.switcher_cinematic_animating = true;
                }
            }

            Message::SwitcherCinematicLeave(offset) => {
                if self.switcher_cinematic_hover == Some(offset) {
                    self.switcher_cinematic_hover = None;
                    self.switcher_cinematic_animating = true;
                }
            }

            Message::SwitcherAnimTick => {
                if self.switcher_window_id.is_some()
                    && self.config.switcher_style == crate::config::SwitcherStyle::Cinematic
                {
                    let mut any_animating = false;

                    // 1. Decay slide offset smoothly towards 0.0
                    if self.switcher_cinematic_slide.abs() > 0.5 {
                        self.switcher_cinematic_slide *= 0.87;
                        if self.switcher_cinematic_slide.abs() <= 0.5 {
                            self.switcher_cinematic_slide = 0.0;
                        }
                        any_animating = true;
                    }

                    // 2. Re-evaluate hit test if cursor is on window
                    if let Some(pos) = self.switcher_cursor_position {
                        let pool_len = self.switcher_pool().len();
                        let new_hover = crate::ui::switcher::cinematic_hit_test(
                            self.switcher_window_width,
                            self.switcher_window_height,
                            pos.x,
                            pos.y,
                            self.switcher_cinematic_slide,
                            &self.switcher_cinematic_scales,
                            pool_len,
                        );
                        if self.switcher_cinematic_hover != new_hover {
                            self.switcher_cinematic_hover = new_hover;
                            any_animating = true;
                        }
                    }

                    // 3. Lerp scales towards hover targets
                    for (i, current) in self.switcher_cinematic_scales.iter_mut().enumerate() {
                        let offset = (i as i32) - 3;
                        let target = if self.switcher_cinematic_hover == Some(offset) { 1.0 } else { 0.0 };
                        let (next, animating) = crate::ui::switcher::cinematic_lerp_step(*current, target);
                        *current = next;
                        if animating {
                            any_animating = true;
                        }
                    }

                    self.switcher_cinematic_animating = any_animating;
                } else {
                    self.switcher_cinematic_animating = false;
                }
            }

            Message::SwitcherEdgeScrollTick => {
                if self.switcher_window_id.is_some() && self.config.switcher_style == crate::config::SwitcherStyle::Honeycomb {
                    let pool_len = self.switcher_pool().len();
                    let rows = crate::ui::switcher::HoneycombLayout::ROWS_PER_COL;
                    let total_cols = (pool_len + rows - 1) / rows;
                    let sel_col = self.switcher_index / rows;
                    let now = Instant::now();
                    if self.switcher_edge_scroll_dir > 0 && sel_col + 1 < total_cols {
                        self.switcher_last_edge_scroll = Some(now);
                        return Task::done(cosmic::Action::App(Message::SwitcherNextCol));
                    } else if self.switcher_edge_scroll_dir < 0 && sel_col > 0 {
                        self.switcher_last_edge_scroll = Some(now);
                        return Task::done(cosmic::Action::App(Message::SwitcherPrevCol));
                    }
                }
            }

            Message::SwitcherResized { window, size } => {
                if self.switcher_window_id == Some(window) {
                    self.switcher_window_width = size.width;
                    self.switcher_window_height = size.height;

                    // Dynamically identify which monitor this surface was mapped to
                    if let Some(matched) = self.outputs.iter().find(|o| {
                        (o.width as f32 - size.width).abs() < 4.0 && (o.height as f32 - size.height).abs() < 4.0
                    }) {
                        self.switcher_active_output = Some(matched.name.clone());
                    } else if let Some(matched) = self.outputs.iter().min_by_key(|o| {
                        ((o.width as f32 - size.width).abs() + (o.height as f32 - size.height).abs()) as u32
                    }) {
                        self.switcher_active_output = Some(matched.name.clone());
                    }
                }
            }

            Message::SelectTrayClickAction(action) => {
                self.config.tray_click_action = action;
                let _ = self.config.save();
            }

            Message::ToggleSwitcherOnlyFavorites(only_favs) => {
                self.config.switcher_only_favorites = only_favs;
                let _ = self.config.save();
            }

            Message::SelectSwitcherPosition(pos) => {
                self.config.switcher_position = pos;
                let _ = self.config.save();
            }

            Message::SelectSwitcherStyle(style) => {
                self.config.switcher_style = style;
                let _ = self.config.save();
            }

            Message::CopySwitcherCommand => {
                let msg = self.language.settings_switcher_copied().to_string();
                return Task::batch([
                    cosmic::iced::clipboard::write("aura switcher".to_string()),
                    self.notify(msg),
                ]);
            }

            Message::WindowCloseRequested(id) => {
                if self.switcher_window_id == Some(id) || self.closing_switcher_window_id == Some(id) {
                    self.switcher_window_id = None;
                    self.closing_switcher_window_id = Some(id);
                    self.switcher_cursor_position = None;
                    self.switcher_edge_scroll_dir = 0;
                    self.switcher_last_edge_scroll = None;
                    self.switcher_active_output = None;
                    return Task::done(cosmic::Action::Cosmic(cosmic::app::Action::Surface(destroy_layer_shell(id))));
                }
                if self.core().main_window_id() == Some(id) {
                    self.is_window_open = false;
                    self.core_mut().set_main_window_id(None);
                    self.core_mut().window.is_maximized = false;
                    self.core_mut().window.sharp_corners = false;
                    if self.config.keep_running_on_close {
                        return cosmic::iced::window::close(id);
                    } else {
                        std::process::exit(0);
                    }
                }
                return cosmic::iced::window::close(id);
            }

            Message::WindowClosed(id) => {
                if self.switcher_window_id == Some(id) {
                    self.switcher_window_id = None;
                    self.switcher_cursor_position = None;
                    self.switcher_edge_scroll_dir = 0;
                    self.switcher_last_edge_scroll = None;
                }
                if self.closing_switcher_window_id == Some(id) {
                    self.closing_switcher_window_id = None;
                }
                if self.core().main_window_id() == Some(id) {
                    self.is_window_open = false;
                    self.core_mut().set_main_window_id(None);
                    self.core_mut().window.is_maximized = false;
                    self.core_mut().window.sharp_corners = false;
                }
                return Task::none();
            }

            Message::NextWallpaper => {
                let pool = self.rotation_pool();
                if !pool.is_empty() {
                    let next_idx = if self.config.order == "random" {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as usize;
                        if pool.len() > 1 {
                            let mut candidate = now % pool.len();
                            if candidate == self.config.seq_index % pool.len() {
                                candidate = (candidate + 1) % pool.len();
                            }
                            candidate
                        } else {
                            0
                        }
                    } else {
                        (self.config.seq_index + 1) % pool.len()
                    };
                    let video_path = pool[next_idx].clone();
                    self.config.seq_index = next_idx;
                    let _ = self.config.save();
                    let output = self.selected_output.clone();
                    return Task::done(cosmic::Action::App(Message::ApplyWallpaper {
                        video_path,
                        output,
                    }));
                }
            }

            Message::PrevWallpaper => {
                let pool = self.rotation_pool();
                if !pool.is_empty() {
                    let prev_idx = if self.config.seq_index == 0 || self.config.seq_index >= pool.len() {
                        pool.len() - 1
                    } else {
                        self.config.seq_index - 1
                    };
                    let video_path = pool[prev_idx].clone();
                    self.config.seq_index = prev_idx;
                    let _ = self.config.save();
                    let output = self.selected_output.clone();
                    return Task::done(cosmic::Action::App(Message::ApplyWallpaper {
                        video_path,
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
                self.last_update_check = Some(Instant::now());
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
                            if self.is_window_open {
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
                let home = std::env::var("HOME").unwrap_or_default();
                let local_bin = std::path::PathBuf::from(&home).join(".local/bin/aura");
                let exe_path = if local_bin.is_file() {
                    local_bin.to_string_lossy().to_string()
                } else if let Ok(current) = std::env::current_exe() {
                    let mut s = current.to_string_lossy().to_string();
                    if s.ends_with(" (deleted)") {
                        s = s.trim_end_matches(" (deleted)").to_string();
                    }
                    s
                } else {
                    "aura".to_string()
                };

                // Detach and delay slightly (400ms) to allow the current instance to fully
                // disconnect from D-Bus and Wayland, avoiding single-instance collision
                let _ = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(format!("sleep 0.4 && exec \"{}\" &", exe_path))
                    .spawn();

                std::process::exit(0);
            }

            Message::ApplyWallpaper { video_path, output } => {
                let path_str = video_path.to_string_lossy().to_string();

                // If this wallpaper is already running on this output and not paused, do nothing to avoid redundant respawns and toast spam
                if self.config.wallpapers.get(&output).map(|s| s.as_str()) == Some(&path_str) && !self.is_paused {
                    return Task::none();
                }

                let scaling = self.config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());
                let mute = self.config.mute;
                let volume = self.config.volume;
                let hwdec = self.config.hwdec.clone();

                match self.engine.set_wallpaper(&output, &path_str, &scaling, mute, volume, &hwdec, self.config.auto_pause, &self.config.gpu_preference) {
                    Ok(_) => {
                        self.config.wallpapers.insert(output.clone(), path_str.clone());
                        self.config.current = Some(path_str.clone());
                        let _ = self.config.save();
                        self.is_paused = false;

                        let file_name = video_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Video".into());
                        self.tray_controller.update_state(file_name.clone(), false, true);

                        // Live sync with quick switcher HUD if it is open
                        if self.switcher_window_id.is_some() {
                            let pool = self.switcher_pool();
                            if let Some(pos) = pool.iter().position(|v| v.path == video_path) {
                                self.switcher_index = pos;
                            }
                        }

                        // COSMIC dynamic accent theme and appearance mode
                        if self.config.auto_theme || self.config.theme_mode != crate::config::ThemeMode::Manual {
                            if let Some(thumb) = self.videos.iter().find(|v| v.path == video_path).and_then(|v| v.thumb_path.as_ref()) {
                                apply_cosmic_theme(thumb, self.config.auto_theme, self.config.theme_mode);
                            } else if let Some(ext) = video_path.extension().and_then(|e| e.to_str()) {
                                if crate::scanner::is_supported_wallpaper_ext(ext) {
                                    apply_cosmic_theme(&video_path, self.config.auto_theme, self.config.theme_mode);
                                }
                            }
                        }

                        let status_msg = self.language.status_applied(&output, &file_name);
                        self.status_message = Some(status_msg.clone());
                        return self.notify_applied(
                            status_msg,
                            self.language.toast_show_in_files(),
                            Message::OpenWallpapersFolder,
                        );
                    }
                    Err(e) => {
                        let err_text = e.to_string();
                        eprintln!("[Aura] Fallo al aplicar fondo: {}", err_text);
                        self.status_message = Some(format!("Error: {}", err_text));
                        self.status_timer = 8;
                        return self.notify(format!("⚠️ {}", err_text));
                    }
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
                if !self.engine.set_scaling(&output, &scaling) {
                    if let Some(path) = self.config.wallpapers.get(&output).cloned() {
                        let _ = self.engine.set_wallpaper(&output, &path, &scaling, self.config.mute, self.config.volume, &self.config.hwdec, self.config.auto_pause, &self.config.gpu_preference);
                    }
                }
                return self.set_status(self.language.status_scaling_changed(&output, &scaling));
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
                if active || self.config.theme_mode != crate::config::ThemeMode::Manual {
                    if let Some(current_str) = &self.config.current {
                        let cur_path = PathBuf::from(current_str);
                        if let Some(thumb) = self.videos.iter().find(|v| v.path == cur_path).and_then(|v| v.thumb_path.as_ref()) {
                            apply_cosmic_theme(thumb, self.config.auto_theme, self.config.theme_mode);
                        } else if let Some(ext) = cur_path.extension().and_then(|e| e.to_str()) {
                            if crate::scanner::is_supported_wallpaper_ext(ext) {
                                apply_cosmic_theme(&cur_path, self.config.auto_theme, self.config.theme_mode);
                            }
                        }
                    }
                }
            }

            Message::SelectThemeMode(mode) => {
                self.config.theme_mode = mode;
                self.config.auto_dark = mode == crate::config::ThemeMode::Auto;
                let _ = self.config.save();

                match mode {
                    crate::config::ThemeMode::Dark => {
                        crate::theme::apply_cosmic_mode(true);
                        self.status_message = Some(self.language.status_theme_mode_dark().into());
                    }
                    crate::config::ThemeMode::Light => {
                        crate::theme::apply_cosmic_mode(false);
                        self.status_message = Some(self.language.status_theme_mode_light().into());
                    }
                    crate::config::ThemeMode::Auto => {
                        if let Some(current_str) = &self.config.current {
                            let cur_path = PathBuf::from(current_str);
                            if let Some(thumb) = self.videos.iter().find(|v| v.path == cur_path).and_then(|v| v.thumb_path.as_ref()) {
                                apply_cosmic_theme(thumb, self.config.auto_theme, self.config.theme_mode);
                            } else if let Some(ext) = cur_path.extension().and_then(|e| e.to_str()) {
                                if crate::scanner::is_supported_wallpaper_ext(ext) {
                                    apply_cosmic_theme(&cur_path, self.config.auto_theme, self.config.theme_mode);
                                }
                            }
                        }
                        self.status_message = Some(self.language.status_theme_mode_auto().into());
                    }
                    crate::config::ThemeMode::Manual => {
                        self.status_message = Some(self.language.status_theme_mode_manual().into());
                    }
                }
                self.status_timer = 5;
            }

            Message::ToggleAutoDark(active) => {
                let mode = if active {
                    crate::config::ThemeMode::Auto
                } else {
                    crate::config::ThemeMode::Manual
                };
                return self.update(Message::SelectThemeMode(mode));
            }

            Message::ToggleSmartPause(active) => {
                self.config.smart_pause = active;
                let _ = self.config.save();
            }

            Message::ToggleAutoPause(active) => {
                self.config.auto_pause = active;
                let _ = self.config.save();
                for (output, path) in self.config.wallpapers.clone() {
                    let sc = self.config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());
                    let _ = self.engine.set_wallpaper(&output, &path, &sc, self.config.mute, self.config.volume, &self.config.hwdec, active, &self.config.gpu_preference);
                }
            }

            Message::ToggleKeepRunningOnClose(active) => {
                self.config.keep_running_on_close = active;
                let _ = self.config.save();
            }

            Message::ToggleMute(mute) => {
                self.config.mute = mute;
                let _ = self.config.save();
                self.engine.set_mute(None, mute);
                self.status_message = Some(if mute { self.language.status_muted().into() } else { self.language.status_unmuted().into() });
                self.status_timer = 5;
            }

            Message::RemoveFolder(folder) => {
                self.config.dirs.retain(|d| d != &folder);
                let _ = self.config.save();
                self.rescan_and_update();
                self.status_message = Some(self.language.status_folder_removed(&folder));
                self.status_timer = 5;
            }

            Message::RefreshLibrary => {
                self.rescan_and_update();
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
                self.library_limit = 48;
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
                self.rescan_and_update();
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
                self.rescan_and_update();
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
                    self.rescan_and_update();
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
                self.handle_topology_change(current_outputs);
            }

            Message::OutputsUpdated(new_outs) => {
                self.handle_topology_change(new_outs);
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
                let pool = self.rotation_pool();
                if !pool.is_empty() && self.config.rotation {
                    let next_idx = if self.config.order == "random" {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as usize;
                        if pool.len() > 1 {
                            let mut candidate = now % pool.len();
                            if candidate == self.config.seq_index % pool.len() {
                                candidate = (candidate + 1) % pool.len();
                            }
                            candidate
                        } else {
                            0
                        }
                    } else {
                        (self.config.seq_index + 1) % pool.len()
                    };
                    let video_path = pool[next_idx].clone();
                    self.config.seq_index = next_idx;
                    let _ = self.config.save();
                    let output = self.selected_output.clone();
                    return Task::done(cosmic::Action::App(Message::ApplyWallpaper {
                        video_path,
                        output,
                    }));
                }
            }

            Message::SelectLibraryFilter(filter) => {
                self.library_filter = filter;
                self.library_limit = 48;
            }

            Message::LoadMoreLibraryWallpapers => {
                self.library_limit += 48;
            }

            Message::ToggleFavorite(path) => {
                let path_str = path.to_string_lossy().to_string();
                if self.favorites_set.contains(&path_str) {
                    self.favorites_set.remove(&path_str);
                } else {
                    self.favorites_set.insert(path_str.clone());
                }
                self.config.toggle_favorite(&path_str);
                self.update_category_counts();
                let _ = self.config.save();
            }

            Message::ToggleLibrarySort => {
                self.library_sort = match self.library_sort {
                    LibrarySort::Newest => LibrarySort::Oldest,
                    LibrarySort::Oldest => LibrarySort::Newest,
                };
                self.config.library_sort = match self.library_sort {
                    LibrarySort::Newest => "newest".into(),
                    LibrarySort::Oldest => "oldest".into(),
                };
                self.sort_videos();
                self.library_limit = 48;
                let _ = self.config.save();
            }

            Message::SelectLibrarySort(sort) => {
                self.library_sort = sort;
                self.config.library_sort = match sort {
                    LibrarySort::Newest => "newest".into(),
                    LibrarySort::Oldest => "oldest".into(),
                };
                self.sort_videos();
                self.library_limit = 48;
                let _ = self.config.save();
            }

            Message::SelectExploreSource(source) => {
                self.explore_source = source;
                let need_fetch = match source {
                    OnlineSource::MotionBGS => self.motionbgs_wallpapers.is_empty(),
                    OnlineSource::Bing => self.bing_wallpapers.is_empty(),
                    OnlineSource::Wallhaven => self.wallhaven_wallpapers.is_empty(),
                    OnlineSource::Minimalistic => self.minimalistic_all_wallpapers.is_empty(),
                };
                if need_fetch && !self.explore_loading {
                    return Task::done(cosmic::Action::App(Message::FetchOnlineWallpapers(source)));
                }
            }

            Message::FetchOnlineWallpapers(source) => {
                self.explore_loading = true;
                self.explore_error = None;
                let client = self.http_client.clone();
                let motion_cat = self.motionbgs_category.clone();
                let motion_res = self.motionbgs_resolution.clone();
                let motion_search = if self.motionbgs_search.trim().is_empty() {
                    None
                } else {
                    Some(self.motionbgs_search.clone())
                };
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
                            OnlineSource::MotionBGS => {
                                fetch_motionbgs_wallpapers(&client, 1, motion_search.as_deref(), &motion_cat, &motion_res)
                                    .await
                                    .map(|items| (source, items))
                            }
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
                            OnlineSource::Minimalistic => {
                                fetch_minimalistic_wallpapers(&client)
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
                        let items_to_thumb = match source {
                            OnlineSource::MotionBGS => {
                                self.motionbgs_wallpapers = items.clone();
                                items
                            }
                            OnlineSource::Bing => {
                                self.bing_wallpapers = items.clone();
                                items
                            }
                            OnlineSource::Wallhaven => {
                                self.wallhaven_wallpapers = items.clone();
                                items
                            }
                            OnlineSource::Minimalistic => {
                                self.minimalistic_all_wallpapers = items;
                                self.minimalistic_page = 1;
                                self.apply_minimalistic_filter()
                            }
                        };
                        self.explore_error = None;
                        return self.queue_thumbnails(&items_to_thumb);
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
                    OnlineSource::MotionBGS => {
                        self.motionbgs_page += 1;
                        let page = self.motionbgs_page;
                        let cat = self.motionbgs_category.clone();
                        let res = self.motionbgs_resolution.clone();
                        let search = if self.motionbgs_search.trim().is_empty() {
                            None
                        } else {
                            Some(self.motionbgs_search.clone())
                        };

                        return Task::perform(
                            async move {
                                fetch_motionbgs_wallpapers(&client, page, search.as_deref(), &cat, &res)
                                    .await
                                    .map(|items| (source, items))
                            },
                            |res| cosmic::Action::App(Message::OnlineMoreWallpapersFetched(res)),
                        );
                    }
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
                    OnlineSource::Minimalistic => {
                        self.explore_loading_more = false;
                        self.minimalistic_page += 1;
                        let paged = self.apply_minimalistic_filter();
                        return self.queue_thumbnails(&paged);
                    }
                }
            }

            Message::OnlineMoreWallpapersFetched(result) => {
                self.explore_loading_more = false;
                match result {
                    Ok((source, items)) => {
                        let mut new_items = Vec::new();
                        match source {
                            OnlineSource::MotionBGS => {
                                for item in items {
                                    if !self.motionbgs_wallpapers.iter().any(|e| e.id == item.id) {
                                        new_items.push(item.clone());
                                        self.motionbgs_wallpapers.push(item);
                                    }
                                }
                            }
                            OnlineSource::Bing => {
                                for item in items {
                                    if !self.bing_wallpapers.iter().any(|e| e.id == item.id) {
                                        new_items.push(item.clone());
                                        self.bing_wallpapers.push(item);
                                    }
                                }
                            }
                            OnlineSource::Wallhaven => {
                                for item in items {
                                    if !self.wallhaven_wallpapers.iter().any(|e| e.id == item.id) {
                                        new_items.push(item.clone());
                                        self.wallhaven_wallpapers.push(item);
                                    }
                                }
                            }
                            OnlineSource::Minimalistic => {}
                        }

                        return self.queue_thumbnails(&new_items);
                    }
                    Err(err) => {
                        match self.explore_source {
                            OnlineSource::MotionBGS => {
                                if self.motionbgs_page > 1 { self.motionbgs_page -= 1; }
                            }
                            OnlineSource::Bing => {
                                if self.bing_page > 1 { self.bing_page -= 1; }
                            }
                            OnlineSource::Wallhaven => {
                                if self.wallhaven_page > 1 { self.wallhaven_page -= 1; }
                            }
                            OnlineSource::Minimalistic => {
                                if self.minimalistic_page > 1 { self.minimalistic_page -= 1; }
                            }
                        }
                        self.status_message = Some(format!("Error: {}", err));
                        self.status_timer = 5;
                    }
                }
            }

            Message::SelectMotionbgsCategory(cat) => {
                let search_cleared = !self.motionbgs_search.is_empty();
                self.motionbgs_search.clear();
                if self.motionbgs_category != cat || search_cleared {
                    self.motionbgs_category = cat;
                    self.motionbgs_page = 1;
                    self.motionbgs_wallpapers.clear();
                    return Task::done(cosmic::Action::App(Message::FetchOnlineWallpapers(OnlineSource::MotionBGS)));
                }
            }

            Message::SelectMotionbgsResolution(res) => {
                if self.motionbgs_resolution != res {
                    self.motionbgs_resolution = res;
                    self.motionbgs_page = 1;
                    self.motionbgs_wallpapers.clear();
                    return Task::done(cosmic::Action::App(Message::FetchOnlineWallpapers(OnlineSource::MotionBGS)));
                }
            }

            Message::MotionbgsSearchChanged(q) => {
                self.motionbgs_search = q;
            }

            Message::SubmitMotionbgsSearch => {
                self.motionbgs_category = "all".into();
                self.motionbgs_page = 1;
                self.motionbgs_wallpapers.clear();
                return Task::done(cosmic::Action::App(Message::FetchOnlineWallpapers(OnlineSource::MotionBGS)));
            }

            Message::ClearMotionbgsSearch => {
                if !self.motionbgs_search.is_empty() {
                    self.motionbgs_search.clear();
                    self.motionbgs_page = 1;
                    self.motionbgs_wallpapers.clear();
                    return Task::done(cosmic::Action::App(Message::FetchOnlineWallpapers(OnlineSource::MotionBGS)));
                }
            }

            Message::SelectWallhavenCategory(cat) => {
                let search_cleared = !self.wallhaven_search.is_empty();
                self.wallhaven_search.clear();
                if self.wallhaven_category != cat || search_cleared {
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

            Message::ClearWallhavenSearch => {
                if !self.wallhaven_search.is_empty() {
                    self.wallhaven_search.clear();
                    self.wallhaven_page = 1;
                    self.wallhaven_wallpapers.clear();
                    return Task::done(cosmic::Action::App(Message::FetchOnlineWallpapers(OnlineSource::Wallhaven)));
                }
            }

            Message::MinimalisticSearchChanged(q) => {
                self.minimalistic_search = q;
                self.minimalistic_page = 1;
                let paged = self.apply_minimalistic_filter();
                return self.queue_thumbnails(&paged);
            }

            Message::ClearMinimalisticSearch => {
                if !self.minimalistic_search.is_empty() {
                    self.minimalistic_search.clear();
                    self.minimalistic_page = 1;
                    let paged = self.apply_minimalistic_filter();
                    return self.queue_thumbnails(&paged);
                }
            }

            Message::OnlineThumbLoaded { id, path } => {
                self.online_thumbs.insert(id, path);
            }

            Message::CancelOnlineDownload(id) => {
                if let Some(flag) = self.download_cancels.remove(&id) {
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                self.downloading_online_ids.remove(&id);
                self.download_progress.remove(&id);
                if self.pending_auto_apply_id.as_deref() == Some(&id) {
                    self.pending_auto_apply_id = None;
                }
            }

            Message::DownloadOnlineWallpaper { item, auto_apply } => {
                self.downloading_online_ids.insert(item.id.clone());
                self.download_progress.insert(item.id.clone(), (0, None, 0.0));
                let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                self.download_cancels.insert(item.id.clone(), cancel_flag.clone());
                if auto_apply {
                    self.pending_auto_apply_id = Some(item.id.clone());
                }
                let client = self.http_client.clone();
                let id = item.id.clone();
                let url = item.full_url.clone();
                let dest = item.local_wallpaper_path();
                let thumb_src = item.local_thumb_path();

                let stream = cosmic::iced::stream::channel(50, move |mut output: cosmic::iced::futures::channel::mpsc::Sender<cosmic::Action<Message>>| {
                    async move {
                        use cosmic::iced::futures::SinkExt;
                        let (tx, mut rx) = tokio::sync::mpsc::channel::<(u64, Option<u64>, f32)>(30);
                        let client_task = client.clone();
                        let url_task = url.clone();
                        let dest_task = dest.clone();
                        let cancel_task = cancel_flag.clone();

                        let dl_handle = tokio::spawn(async move {
                            download_to_file_with_progress(&client_task, &url_task, &dest_task, Some(cancel_task), move |down, tot, pct| {
                                let _ = tx.try_send((down, tot, pct));
                            }).await
                        });

                        // Forward real-time throttled progress updates to Iced UI
                        while let Some((downloaded, total, percent)) = rx.recv().await {
                            let _ = output.send(cosmic::Action::App(Message::DownloadProgressUpdated {
                                id: id.clone(),
                                downloaded,
                                total,
                                percent,
                            })).await;
                        }

                        let download_res = dl_handle.await.unwrap_or_else(|e| Err(format!("Download task error: {}", e)));

                        match download_res {
                            Ok(p) => {
                                let target_thumb = crate::scanner::thumbs::thumb_path_for_video(&p);
                                let is_image = p.extension()
                                    .and_then(|e| e.to_str())
                                    .map(|ext| crate::scanner::IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                                    .unwrap_or(false);

                                let generated = if is_image {
                                    let _ = tokio::fs::remove_file(&target_thumb).await;
                                    crate::scanner::thumbs::generate_thumbnail(&p).await.is_some()
                                } else {
                                    false
                                };

                                if !generated && thumb_src.exists() && !target_thumb.exists() {
                                    if let Some(parent) = target_thumb.parent() {
                                        let _ = tokio::fs::create_dir_all(parent).await;
                                    }
                                    let _ = tokio::fs::copy(&thumb_src, &target_thumb).await;
                                }

                                let _ = output.send(cosmic::Action::App(Message::OnlineWallpaperDownloaded {
                                    id: id.clone(),
                                    path: p,
                                    auto_apply,
                                })).await;
                            }
                            Err(err) => {
                                let _ = output.send(cosmic::Action::App(Message::OnlineWallpaperDownloadFailed {
                                    id: id.clone(),
                                    error: err,
                                })).await;
                            }
                        }
                    }
                });

                return cosmic::task::stream(stream);
            }

            Message::DownloadProgressUpdated { id, downloaded, total, percent } => {
                self.download_progress.insert(id, (downloaded, total, percent));
            }

            Message::OnlineWallpaperDownloaded { id, path, auto_apply } => {
                self.download_cancels.remove(&id);
                self.downloading_online_ids.remove(&id);
                self.download_progress.remove(&id);
                self.rescan_and_update();
                let _ = self.config.save();

                let mut tasks = Vec::new();
                if let Some(item) = self.videos.iter().find(|v| v.path == path) {
                    if item.thumb_path.is_none() {
                        let vp = path.clone();
                        let vp_for_msg = path.clone();
                        let is_video = vp.extension()
                            .and_then(|e| e.to_str())
                            .map(|ext| crate::scanner::VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                            .unwrap_or(false);
                        if is_video {
                            tasks.push(Task::perform(
                                async move {
                                    crate::scanner::thumbs::generate_thumbnail(&vp).await
                                },
                                move |thumb_opt| {
                                    if let Some(tp) = thumb_opt {
                                        cosmic::Action::App(Message::ThumbnailGenerated {
                                            video_path: vp_for_msg,
                                            thumb_path: tp,
                                        })
                                    } else {
                                        cosmic::Action::None
                                    }
                                },
                            ));
                        }
                    }
                }

                if auto_apply {
                    if self.pending_auto_apply_id.as_deref() == Some(&id) {
                        self.pending_auto_apply_id = None;
                        tasks.push(Task::done(cosmic::Action::App(Message::ApplyDownloadedOnlineWallpaper(path))));
                    }
                } else {
                    let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let msg_text = format!("{}: {}", self.language.explore_toast_downloaded(), fname);
                    tasks.push(self.notify_with_action(
                        msg_text,
                        self.language.toast_show_in_files(),
                        Message::OpenWallpapersFolder,
                    ));
                }

                if !tasks.is_empty() {
                    return Task::batch(tasks);
                }
            }

            Message::OnlineWallpaperDownloadFailed { id, error } => {
                self.download_cancels.remove(&id);
                let was_downloading = self.downloading_online_ids.remove(&id);
                self.download_progress.remove(&id);
                if self.pending_auto_apply_id.as_deref() == Some(&id) {
                    self.pending_auto_apply_id = None;
                }
                if was_downloading && !error.contains("canceled by user") {
                    self.status_message = Some(format!("{} {}", self.language.explore_error_prefix(), error));
                    self.status_timer = 5;
                }
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
                self.rescan_and_update();

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

                self.rescan_and_update();

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
            .spacing(10)
            .width(Length::Fill)
            .height(Length::Fill);

        main_col = main_col.push(
            widget::container(content)
                .width(Length::Fill)
                .height(Length::Fill),
        );

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

#[cfg(test)]
mod tests {
    #[test]
    fn test_custom_interval_input_sanitization_and_validation() {
        let input = "abc 45 min!";
        let sanitized: String = input.chars().filter(|c| c.is_ascii_digit()).take(5).collect();
        assert_eq!(sanitized, "45");

        let val = sanitized.parse::<u64>().unwrap();
        assert!((1..=1440).contains(&val));

        // Invalid zero
        let zero_input = "0".chars().filter(|c| c.is_ascii_digit()).take(5).collect::<String>();
        let val_zero = zero_input.parse::<u64>().unwrap();
        assert!(!(1..=1440).contains(&val_zero));

        // Invalid excessive (> 1440)
        let big_input = "1441".chars().filter(|c| c.is_ascii_digit()).take(5).collect::<String>();
        let val_big = big_input.parse::<u64>().unwrap();
        assert!(!(1..=1440).contains(&val_big));
    }

    #[test]
    fn test_switcher_scroll_accumulator_threshold() {
        const TOUCHPAD_THRESHOLD: f32 = 60.0;
        let mut accum = 0.0f32;

        // Small events from gentle touch (e.g. 10 events of 3.0 px)
        for _ in 0..10 {
            accum += 3.0;
        }
        assert_eq!(accum, 30.0);
        // Not yet at threshold
        assert!(accum < TOUCHPAD_THRESHOLD);

        // More events push it over threshold
        accum += 35.0; // 65.0
        assert!(accum >= TOUCHPAD_THRESHOLD);

        // Upon trigger, reset
        accum = 0.0;
        assert_eq!(accum, 0.0);

        // Direction reversal reset: if moving down (+20), then user swipes up (-15)
        accum += 20.0;
        let increment = -15.0;
        if (increment > 0.0 && accum < 0.0) || (increment < 0.0 && accum > 0.0) {
            accum = 0.0;
        }
        accum += increment;
        assert_eq!(accum, -15.0);
    }

    #[test]
    fn test_switcher_edge_scroll_margin_and_clamping() {
        // Test 1366x768 (User's display)
        let width_1366 = 1366.0f32;
        let margin_1366 = (width_1366 * 0.08).clamp(70.0, 130.0);
        assert!((margin_1366 - 109.28).abs() < 0.01);

        // On 1366: cursor near right edge (1320px) should trigger right scroll (+1)
        let pos_right_1366 = 1320.0f32;
        let dir_right_1366 = if pos_right_1366 < margin_1366 {
            -1
        } else if pos_right_1366 > width_1366 - margin_1366 {
            1
        } else {
            0
        };
        assert_eq!(dir_right_1366, 1, "Right edge scroll must trigger on 1366px screen");

        // Test 1920x1080
        let width = 1920.0f32;
        let edge_margin = (width * 0.08).clamp(70.0, 130.0);
        assert_eq!(edge_margin, 130.0);

        // Cursor at far left (50px)
        let pos_left = 50.0f32;
        let dir_left = if pos_left < edge_margin {
            -1
        } else if pos_left > width - edge_margin {
            1
        } else {
            0
        };
        assert_eq!(dir_left, -1);

        // Cursor at center (960px)
        let pos_center = 960.0f32;
        let dir_center = if pos_center < edge_margin {
            -1
        } else if pos_center > width - edge_margin {
            1
        } else {
            0
        };
        assert_eq!(dir_center, 0);

        // Cursor at far right (1850px)
        let pos_right = 1850.0f32;
        let dir_right = if pos_right < edge_margin {
            -1
        } else if pos_right > width - edge_margin {
            1
        } else {
            0
        };
        assert_eq!(dir_right, 1);

        // Boundary checks for a 10-item library (4 columns: 0, 1, 2, 3)
        let n = 10usize;
        let total_cols = (n + 2) / 3;
        assert_eq!(total_cols, 4);

        // At column 0: can step right, but not left
        let sel_col_0 = 0usize;
        assert!(!(sel_col_0 > 0), "Should not scroll left past column 0");
        assert!(sel_col_0 + 1 < total_cols, "Should scroll right from column 0");

        // At column 3 (last column): can step left, but not right
        let sel_col_last = 3usize;
        assert!(sel_col_last > 0, "Should scroll left from last column");
        assert!(!(sel_col_last + 1 < total_cols), "Should not scroll right past last column");
    }
}

