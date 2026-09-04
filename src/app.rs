use crate::config::Config;
use crate::engine::{detect_outputs, MonitorOutput, WallpaperEngine};
use crate::scanner::{scan_directories, thumbs::generate_thumbnail, VideoItem};
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
    SelectScaling { output: String, scaling: String },
    ToggleAutostart(bool),
    ToggleAutoTheme(bool),
    ToggleAutoDark(bool),
    ToggleMute(bool),
    SetHwdec(String),
    ToggleRotation(bool),
    SetInterval(u64),
    RemoveFolder(String),
    RefreshLibrary,
    ThumbnailGenerated { video_path: PathBuf, thumb_path: PathBuf },
    SearchChanged(String),
    RotationTick,
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
        let videos = scan_directories(&config.dirs);

        // Spawn async non-blocking background thumbnail generation for un-cached videos
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
        let is_running = !self.config.wallpapers.is_empty();
        vec![
            Element::from(
                widget::button::destructive("⏹ Detener Fondo")
                    .on_press_maybe(if is_running { Some(Message::StopWallpaper(None)) } else { None })
            )
        ]
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        if self.config.rotation && self.config.interval > 0 {
            cosmic::iced::time::every(Duration::from_secs(self.config.interval * 60))
                .map(|_| Message::RotationTick)
        } else {
            Subscription::none()
        }
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
                    self.config.current = Some(path_str);
                    let _ = self.config.save();

                    // Dynamic COSMIC accent auto-theming
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

            Message::StopWallpaper(output) => {
                if let Some(out) = output {
                    self.engine.stop_output(&out);
                    self.config.wallpapers.remove(&out);
                } else {
                    self.engine.stop_all();
                    self.config.wallpapers.clear();
                    self.config.current = None;
                }
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
                }
            }

            Message::ToggleAutostart(active) => {
                self.autostart_active = active;
                if active {
                    let _ = self.engine.write_autostart(&self.config.wallpapers, &self.config.scaling, self.config.mute, &self.config.hwdec);
                } else {
                    let _ = WallpaperEngine::remove_autostart();
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

            Message::ToggleMute(mute) => {
                self.config.mute = mute;
                let _ = self.config.save();
                for (output, path) in self.config.wallpapers.clone() {
                    let sc = self.config.scaling.get(&output).cloned().unwrap_or_else(|| "fit".into());
                    let _ = self.engine.set_wallpaper(&output, &path, &sc, mute, &self.config.hwdec);
                }
            }

            Message::SetHwdec(hwdec) => {
                self.config.hwdec = hwdec;
                let _ = self.config.save();
            }

            Message::ToggleRotation(rot) => {
                self.config.rotation = rot;
                let _ = self.config.save();
            }

            Message::SetInterval(mins) => {
                self.config.interval = mins;
                let _ = self.config.save();
            }

            Message::RemoveFolder(folder) => {
                self.config.dirs.retain(|d| d != &folder);
                let _ = self.config.save();
                self.videos = scan_directories(&self.config.dirs);
            }

            Message::RefreshLibrary => {
                self.videos = scan_directories(&self.config.dirs);
            }

            Message::ThumbnailGenerated { video_path, thumb_path } => {
                if let Some(item) = self.videos.iter_mut().find(|v| v.path == video_path) {
                    item.thumb_path = Some(thumb_path);
                }
            }

            Message::SearchChanged(q) => {
                self.search_query = q;
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

        Element::from(
            widget::container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(16)
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
            let empty_msg = widget::column::with_capacity(3)
                .spacing(12)
                .align_x(Horizontal::Center)
                .push(widget::text::title2("No se encontraron fondos de pantalla"))
                .push(widget::text::body("Agrega carpetas con videos (MP4, WebM, MKV) en los Ajustes."))
                .push(widget::button::standard("Refrescar Biblioteca").on_press(Message::RefreshLibrary));

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

                let mut card_content = widget::column::with_capacity(4).spacing(8).padding(12);

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
                    let placeholder = widget::container(widget::text::body("Generando miniatura..."))
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
                    widget::button::standard("Aplicar")
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
                .spacing(16)
                .push(search_bar)
                .push(scroll)
        )
    }

    fn view_monitors(&self) -> Element<'_, Message> {
        let mut col = widget::column::with_capacity(self.outputs.len() + 2)
            .spacing(16)
            .width(Length::Fill);

        col = col.push(widget::text::title2("Monitores y Pantallas Detectadas"));
        col = col.push(widget::text::body("Configura fondos independientes y modo de escala por cada monitor."));

        for monitor in &self.outputs {
            let active_wall = self.config.wallpapers.get(&monitor.name);
            let active_scaling = self.config.scaling.get(&monitor.name).map(|s| s.as_str()).unwrap_or("fit");

            let monitor_box = widget::column::with_capacity(4)
                .spacing(10)
                .padding(16)
                .push(widget::text::title3(format!("🖥️ {} ({})", monitor.name, monitor.resolution)))
                .push(widget::text::body(format!("Fondo actual: {}", active_wall.map(|s| s.as_str()).unwrap_or("Ninguno (Escritorio COSMIC)"))))
                .push(
                    widget::row::with_capacity(5)
                        .spacing(12)
                        .align_y(Alignment::Center)
                        .push(widget::text::body("Escala:"))
                        .push(
                            if active_scaling == "fit" {
                                widget::button::suggested("Ajustar (Fit)")
                            } else {
                                widget::button::standard("Ajustar (Fit)")
                            }.on_press(Message::SelectScaling { output: monitor.name.clone(), scaling: "fit".into() })
                        )
                        .push(
                            if active_scaling == "fill" {
                                widget::button::suggested("Rellenar (Fill)")
                            } else {
                                widget::button::standard("Rellenar (Fill)")
                            }.on_press(Message::SelectScaling { output: monitor.name.clone(), scaling: "fill".into() })
                        )
                        .push(
                            if active_scaling == "stretch" {
                                widget::button::suggested("Estirar (Stretch)")
                            } else {
                                widget::button::standard("Estirar (Stretch)")
                            }.on_press(Message::SelectScaling { output: monitor.name.clone(), scaling: "stretch".into() })
                        )
                        .push(
                            widget::button::destructive("Detener")
                                .on_press(Message::StopWallpaper(Some(monitor.name.clone())))
                        )
                );

            col = col.push(widget::container(monitor_box).width(Length::Fill));
        }

        Element::from(col)
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let mut col = widget::column::with_capacity(3)
            .spacing(20)
            .width(Length::Fill);

        col = col.push(widget::text::title2("Ajustes del Sistema"));

        let general_section = widget::column::with_capacity(5)
            .spacing(14)
            .padding(16)
            .push(widget::text::title3("Integración con COSMIC"))
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.autostart_active).on_toggle(Message::ToggleAutostart))
                    .push(widget::text::body("Iniciar fondo animado automáticamente al encender el equipo"))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.auto_theme).on_toggle(Message::ToggleAutoTheme))
                    .push(widget::text::body("Auto-Tema COSMIC (Sincronizar el color de acento del sistema con el fondo)"))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.auto_dark).on_toggle(Message::ToggleAutoDark))
                    .push(widget::text::body("Cambiar automáticamente entre Modo Oscuro y Modo Claro según la luminosidad"))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.mute).on_toggle(Message::ToggleMute))
                    .push(widget::text::body("Silenciar el audio de los videos (recomendado)"))
            );

        col = col.push(widget::container(general_section).width(Length::Fill));

        let mut folders_box = widget::column::with_capacity(self.config.dirs.len() + 1)
            .spacing(10)
            .padding(16);

        folders_box = folders_box.push(widget::text::title3("Carpetas de Fondos"));

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

        Element::from(widget::scrollable(col).height(Length::Fill))
    }
}
