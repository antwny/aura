use crate::config::Config;
use crate::engine::{detect_outputs, MonitorOutput, WallpaperEngine};
use crate::scanner::{scan_directories, thumbs::generate_thumbnail, VideoItem, VIDEO_EXTENSIONS};
use crate::theme::apply_cosmic_theme;

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
    Monitors,
    Settings,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    SelectPage(nav_bar::Id),
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
    OutputsUpdated(Vec<MonitorOutput>),
    PickVideoFile,
    PickFolder,
    FileSelected(Option<PathBuf>),
    FolderSelected(Option<PathBuf>),
    FilesDropped(Vec<PathBuf>),
    DismissStatus,
}

pub struct AuraApp {
    core: Core,
    nav: nav_bar::Model,
    active_page: Page,
    config: Config,
    engine: WallpaperEngine,
    outputs: Vec<MonitorOutput>,
    selected_output: String,
    videos: Vec<VideoItem>,
    search_query: String,
    autostart_active: bool,
    status_message: Option<String>,
    is_paused: bool,
}

impl cosmic::Application for AuraApp {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "io.github.antwny.aura";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text("Biblioteca")
            .data(Page::Library)
            .icon(widget::icon::from_name("video-x-generic"))
            .activate();

        nav.insert()
            .text("Pantallas")
            .data(Page::Monitors)
            .icon(widget::icon::from_name("video-display-symbolic"));

        nav.insert()
            .text("Ajustes")
            .data(Page::Settings)
            .icon(widget::icon::from_name("preferences-system-symbolic"));

        let config = Config::load();
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

        let app = Self {
            core,
            nav,
            active_page: Page::Library,
            config,
            engine: WallpaperEngine::new(),
            outputs,
            selected_output,
            videos,
            search_query: String::new(),
            autostart_active,
            status_message: None,
            is_paused: false,
        };

        (app, Task::batch(tasks))
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        self.nav.activate(id);
        if let Some(&page) = self.nav.active_data::<Page>() {
            self.active_page = page;
        }
        Task::none()
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        vec![
            Element::from(widget::text::title3("Aura")),
            Element::from(widget::text::body(format!("({} fondos)", self.videos.len()))),
        ]
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        vec![
            Element::from(
                widget::button::suggested("+ Añadir Video")
                    .on_press(Message::PickVideoFile)
            ),
            Element::from(
                widget::button::standard("+ Carpeta")
                    .on_press(Message::PickFolder)
            ),
        ]
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let mut subs = Vec::new();

        // 1. Drag & Drop subscription
        subs.push(cosmic::iced::event::listen().map(|event| {
            if let cosmic::iced::Event::Window(cosmic::iced::window::Event::FileDropped(paths)) = event {
                Message::FilesDropped(paths)
            } else {
                Message::DismissStatus
            }
        }));

        // 2. Playlist auto-rotation
        if self.config.rotation && self.config.interval > 0 {
            subs.push(
                cosmic::iced::time::every(Duration::from_secs(self.config.interval * 60))
                    .map(|_| Message::RotationTick)
            );
        }

        // 3. Display hotplug monitor detector (checks every 5 seconds)
        subs.push(
            cosmic::iced::time::every(Duration::from_secs(5))
                .map(|_| Message::HotplugTick)
        );

        // 4. Smart Pause: Game & Fullscreen detection (checks every 3 seconds)
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
                }
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
                    self.status_message = Some(format!("✨ Fondo aplicado en {}: '{}'", output, file_name));

                    // COSMIC dynamic accent theme
                    if self.config.auto_theme {
                        if let Some(thumb) = self.videos.iter().find(|v| v.path == video_path).and_then(|v| v.thumb_path.as_ref()) {
                            apply_cosmic_theme(thumb, self.config.auto_dark);
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
                if now_paused {
                    self.status_message = Some("⏸ Fondo pausado (0% GPU/CPU)".into());
                } else {
                    self.status_message = Some("▶ Fondo reanudado".into());
                }
            }

            Message::StopWallpaper(output) => {
                if let Some(out) = output {
                    self.engine.stop_output(&out);
                    self.config.wallpapers.remove(&out);
                    self.status_message = Some(format!("⏹ Fondo detenido en {}", out));
                } else {
                    self.engine.stop_all();
                    self.config.wallpapers.clear();
                    self.config.current = None;
                    self.status_message = Some("⏹ Todos los fondos han sido detenidos".into());
                }
                self.is_paused = false;
                let _ = self.config.save();
                if self.autostart_active {
                    let _ = self.engine.write_autostart(&self.config.wallpapers, &self.config.scaling, self.config.mute, &self.config.hwdec);
                }
            }

            Message::SelectScaling { output, scaling } => {
                self.config.scaling.insert(output.clone(), scaling.clone());
                let _ = self.config.save();
                if let Some(path) = self.config.wallpapers.get(&output).cloned() {
                    let _ = self.engine.set_wallpaper(&output, &path, &scaling, self.config.mute, &self.config.hwdec);
                    self.status_message = Some(format!("📐 Modo de escala para {} cambiado a '{}'", output, scaling));
                }
            }

            Message::ToggleAutostart(active) => {
                self.autostart_active = active;
                if active {
                    let _ = self.engine.write_autostart(&self.config.wallpapers, &self.config.scaling, self.config.mute, &self.config.hwdec);
                    self.status_message = Some("✅ Inicio automático activado".into());
                } else {
                    let _ = WallpaperEngine::remove_autostart();
                    self.status_message = Some("❌ Inicio automático desactivado".into());
                }
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
                self.status_message = Some(if mute { "🔇 Audio silenciado".into() } else { "🔊 Audio activado".into() });
            }

            Message::RemoveFolder(folder) => {
                self.config.dirs.retain(|d| d != &folder);
                let _ = self.config.save();
                self.videos = scan_directories(&self.config.dirs, &self.config.custom_videos);
                self.status_message = Some(format!("Carpeta eliminada: {}", folder));
            }

            Message::RefreshLibrary => {
                self.videos = scan_directories(&self.config.dirs, &self.config.custom_videos);
                self.status_message = Some("Biblioteca actualizada".into());
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

            Message::DismissStatus => {
                // Keep status until user acts or changes
            }

            // XDG File Dialog: Pick Video
            Message::PickVideoFile => {
                return Task::perform(
                    async {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Seleccionar Video de Fondo")
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
                self.status_message = Some(format!("📁 Video añadido: {}", path.file_name().unwrap_or_default().to_string_lossy()));

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
                return Task::perform(
                    async {
                        let folder = rfd::AsyncFileDialog::new()
                            .set_title("Seleccionar Carpeta con Videos")
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
                self.status_message = Some(format!("📁 Carpeta añadida: {}", folder_str));

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
                    self.status_message = Some("🎯 Video(s) añadidos mediante Drag & Drop!".into());

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
                    self.status_message = Some(format!("🖥️ Topología de pantallas actualizada ({} monitores)", new_count));
                }
            }

            Message::OutputsUpdated(new_outs) => {
                self.outputs = new_outs;
            }

            Message::SmartPauseTick => {
                if self.config.smart_pause && !self.config.wallpapers.is_empty() {
                    // Check if fullscreen game/gamescope/steam app is active
                    let is_gaming = check_if_fullscreen_game_active();
                    if is_gaming && !self.is_paused {
                        self.engine.pause_all();
                        self.is_paused = true;
                        self.status_message = Some("🎮 Modo Juego: Fondo pausado para 100% FPS".into());
                    } else if !is_gaming && self.is_paused {
                        self.engine.resume_all();
                        self.is_paused = false;
                        self.status_message = Some("▶ Juego minimizado: Fondo reanudado".into());
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
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let content: Element<_> = match self.active_page {
            Page::Library => self.view_library(),
            Page::Monitors => self.view_monitors(),
            Page::Settings => self.view_settings(),
        };

        let mut main_col = widget::column::with_capacity(3)
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill);

        // Optional Toast / Status banner
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
            .padding(8)
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
    fn view_library(&self) -> Element<'_, Message> {
        let search_bar = widget::text_input::search_input("Buscar fondos por nombre...", &self.search_query)
            .on_input(Message::SearchChanged)
            .width(Length::Fill);

        let filtered_videos: Vec<&VideoItem> = self.videos
            .iter()
            .filter(|v| {
                if self.search_query.trim().is_empty() {
                    true
                } else {
                    v.name.to_lowercase().contains(&self.search_query.to_lowercase())
                }
            })
            .collect();

        if filtered_videos.is_empty() {
            let empty_msg = widget::column::with_capacity(4)
                .spacing(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title2("Tu Biblioteca de Fondos está vacía"))
                .push(widget::text::body("Arrastra y suelta un archivo .mp4 aquí, o usa los botones para añadir:"))
                .push(
                    widget::row::with_capacity(2)
                        .spacing(12)
                        .push(widget::button::suggested("+ Añadir Video").on_press(Message::PickVideoFile))
                        .push(widget::button::standard("+ Añadir Carpeta").on_press(Message::PickFolder))
                );

            return Element::from(
                widget::container(empty_msg)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
            );
        }

        let mut cards_column = widget::column::with_capacity(filtered_videos.len() / 3 + 1)
            .spacing(16)
            .width(Length::Fill);

        for chunk in filtered_videos.chunks(3) {
            let mut card_row = widget::row::with_capacity(3).spacing(16).width(Length::Fill);
            for video in chunk {
                let is_active = self.config.wallpapers.values().any(|p| p == &video.path.to_string_lossy());

                let mut card_content = widget::column::with_capacity(5).spacing(8).padding(12);

                if let Some(thumb) = &video.thumb_path {
                    let img_btn = widget::button::image(thumb.to_string_lossy().to_string())
                        .width(240.0)
                        .selected(is_active)
                        .on_press(Message::ApplyWallpaper {
                            video_path: video.path.clone(),
                            output: self.selected_output.clone(),
                        });
                    card_content = card_content.push(img_btn);
                } else {
                    let placeholder = widget::container(widget::text::body("Extrayendo fotograma..."))
                        .width(Length::Fixed(240.0))
                        .height(Length::Fixed(135.0))
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center);
                    card_content = card_content.push(placeholder);
                }

                let title = widget::text::body(&video.name).size(14);
                let size = widget::text::caption(&video.size_formatted);
                card_content = card_content.push(title).push(size);

                let apply_btn = if is_active {
                    widget::button::suggested("★ Activo")
                } else {
                    widget::button::standard("▶ Aplicar")
                }.on_press(Message::ApplyWallpaper {
                    video_path: video.path.clone(),
                    output: self.selected_output.clone(),
                });

                card_content = card_content.push(apply_btn);

                let card_container = widget::container(card_content)
                    .width(Length::Fixed(264.0));

                card_row = card_row.push(card_container);
            }
            cards_column = cards_column.push(card_row);
        }

        let scroll = widget::scrollable(cards_column).height(Length::Fill);

        Element::from(
            widget::column::with_capacity(2)
                .spacing(14)
                .push(search_bar)
                .push(scroll)
        )
    }

    fn view_monitors(&self) -> Element<'_, Message> {
        let mut col = widget::column::with_capacity(self.outputs.len() + 3)
            .spacing(18)
            .width(Length::Fill);

        col = col.push(widget::text::title2("Visualizador Interactivo de Pantallas"));
        col = col.push(widget::text::body("Representación a escala de tus pantallas detectadas en COSMIC. Configura fondos y escalado independiente:"));

        // Horizontal Canvas of Virtual Monitors
        let mut monitors_canvas = widget::row::with_capacity(self.outputs.len())
            .spacing(24)
            .align_y(Alignment::End);

        for monitor in &self.outputs {
            let active_wall = self.config.wallpapers.get(&monitor.name);
            let active_scaling = self.config.scaling.get(&monitor.name).map(|s| s.as_str()).unwrap_or("fit");

            // Calculate scale dimensions
            let ar = monitor.aspect_ratio();
            let screen_w = 260.0f32;
            let _screen_h = (screen_w / ar).clamp(130.0, 180.0);

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
                        widget::button::standard(format!("🖥️ {}", monitor.name))
                            .width(screen_w)
                    )
                }
            } else {
                Element::from(
                    widget::button::standard(format!("🖥️ {}\n(Escritorio COSMIC)", monitor.name))
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
                            widget::button::destructive("Detener")
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
            let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Fondo Activo".into());
            let out = self.config.wallpapers.keys().cloned().collect::<Vec<_>>().join(", ");
            (name, format!("🖥️ En: {}", out))
        } else {
            ("Ningún fondo en reproducción".into(), "Escritorio COSMIC estándar".into())
        };

        let play_pause_btn = if self.is_paused {
            widget::button::suggested("▶ Reanudar").on_press_maybe(if has_wallpapers { Some(Message::TogglePause) } else { None })
        } else {
            widget::button::standard("⏸ Pausar").on_press_maybe(if has_wallpapers { Some(Message::TogglePause) } else { None })
        };

        let mute_btn = if self.config.mute {
            widget::button::standard("🔇 Mudo").on_press(Message::ToggleMute(false))
        } else {
            widget::button::suggested("🔊 Audio On").on_press(Message::ToggleMute(true))
        };

        let stop_btn = widget::button::destructive("⏹ Detener Todo")
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
                    .push(widget::text::body(format!("🎬 {}", wall_title)).size(14))
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
                    .push(widget::text::caption(format!("⚡ GPU: {}", self.config.hwdec)))
                    .push(widget::text::caption("Wayland Layer-Shell"))
            );

        Element::from(
            widget::container(bar_content)
                .padding(12)
                .width(Length::Fill)
        )
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let mut col = widget::column::with_capacity(4)
            .spacing(18)
            .width(Length::Fill);

        col = col.push(widget::text::title2("Ajustes del Sistema"));

        // General settings
        let general_section = widget::column::with_capacity(6)
            .spacing(14)
            .padding(16)
            .push(widget::text::title3("Integración y Rendimiento"))
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.autostart_active).on_toggle(Message::ToggleAutostart))
                    .push(widget::text::body("Iniciar fondo animado automáticamente al iniciar sesión en Pop!_OS"))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.smart_pause).on_toggle(Message::ToggleSmartPause))
                    .push(widget::text::body("🎮 Smart Pause: Pausar fondo automáticamente en juegos y ventanas a pantalla completa (0% GPU)"))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.keep_running_on_close).on_toggle(Message::ToggleKeepRunningOnClose))
                    .push(widget::text::body("Mantener el fondo activo al cerrar la aplicación (recomendado)"))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.auto_theme).on_toggle(Message::ToggleAutoTheme))
                    .push(widget::text::body("🎨 Auto-Tema COSMIC: Extraer y aplicar color de acento del fondo en el escritorio"))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.auto_dark).on_toggle(Message::ToggleAutoDark))
                    .push(widget::text::body("🌙 Sincronizar Modo Oscuro / Claro automáticamente según la claridad del video"))
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
                .push(widget::text::title3("Carpetas Monitoreadas").width(Length::Fill))
                .push(widget::button::suggested("+ Añadir Carpeta").on_press(Message::PickFolder))
        );

        for dir in &self.config.dirs {
            let dir_clone = dir.clone();
            let dir_row = widget::row::with_capacity(2)
                .spacing(12)
                .align_y(Alignment::Center)
                .push(widget::text::body(dir).width(Length::Fill))
                .push(
                    widget::button::destructive("Eliminar")
                        .on_press(Message::RemoveFolder(dir_clone))
                );
            folders_box = folders_box.push(dir_row);
        }

        col = col.push(widget::container(folders_box).width(Length::Fill));

        // Persistence info card
        let info_card = widget::column::with_capacity(2)
            .spacing(6)
            .padding(14)
            .push(widget::text::title3("ℹ️ Persistencia Inteligente"))
            .push(widget::text::caption(
                "Aura funciona como centro de mando. Al cerrar esta ventana, tu fondo animado continuará reproduciéndose \
                sin problemas en tu compositor Wayland a través de mpvpaper, liberando el 100% de la memoria de la interfaz."
            ));

        col = col.push(widget::container(info_card).width(Length::Fill));

        Element::from(widget::scrollable(col).height(Length::Fill))
    }
}

fn check_if_fullscreen_game_active() -> bool {
    // Check if heavy 3D titles or gamescope/steam processes are running
    let heavy_processes = ["gamescope", "steam_app", "wine64-preloader", "proton", "heroic", "lutris"];
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.filter_map(|e| e.ok()) {
            if let Ok(cmdline) = std::fs::read_to_string(entry.path().join("cmdline")) {
                for proc_name in heavy_processes {
                    if cmdline.contains(proc_name) {
                        return true;
                    }
                }
            }
        }
    }
    false
}
