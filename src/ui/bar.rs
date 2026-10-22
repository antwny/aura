use crate::app::{AuraApp, BarPopover, Message};
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    fn truncate_bar_title(text: &str, max_chars: usize) -> String {
        if text.chars().count() <= max_chars {
            return text.to_string();
        }
        let mut shortened: String = text.chars().take(max_chars.saturating_sub(1)).collect();
        shortened.push('…');
        shortened
    }

    fn bar_play_pause_icon(
        &self,
        has_wallpapers: bool,
    ) -> widget::button::IconButton<'_, Message> {
        if self.is_paused {
            widget::button::icon(widget::icon::from_name("media-playback-start-symbolic"))
                .class(cosmic::theme::Button::Suggested)
                .tooltip(self.language.bar_resume())
                .on_press_maybe(has_wallpapers.then_some(Message::TogglePause))
        } else {
            widget::button::icon(widget::icon::from_name("media-playback-pause-symbolic"))
                .tooltip(self.language.bar_pause())
                .on_press_maybe(has_wallpapers.then_some(Message::TogglePause))
        }
    }

    fn bar_popover<'a>(
        &'a self,
        content: Element<'a, Message>,
        panel: BarPopover,
        popup: Element<'a, Message>,
    ) -> Element<'a, Message> {
        if self.bar_popover == Some(panel) {
            widget::popover(content)
                .popup(popup)
                .position(cosmic::widget::popover::Position::Top)
                .on_close(Message::CloseBarPopover)
                .into()
        } else {
            content
        }
    }

    fn bar_panel<'a>(&self, title: &str, content: Element<'a, Message>) -> Element<'a, Message> {
        widget::container(
            widget::column::with_capacity(2)
                .spacing(8)
                .push(widget::text::title3(title.to_string()))
                .push(content),
        )
        .width(Length::Fixed(if self.window_width < 480.0 { 250.0 } else { 300.0 }))
        .padding(12)
        .class(cosmic::theme::Container::Card)
        .into()
    }

    fn bar_volume_popover(&self) -> Element<'_, Message> {
        let lang = self.language;
        let volume = self.config.volume;
        let muted = self.config.mute;
        let mute_button = if muted {
            widget::button::suggested(lang.bar_muted())
                .leading_icon(widget::icon::from_name("audio-volume-muted-symbolic"))
                .on_press(Message::ToggleMute(false))
        } else {
            widget::button::standard(lang.bar_audio_active())
                .leading_icon(widget::icon::from_name("audio-volume-high-symbolic"))
                .on_press(Message::ToggleMute(true))
        };

        let content = widget::column::with_capacity(5)
            .spacing(8)
            .push(mute_button.width(Length::Fill))
            .push(
                widget::row::with_capacity(2)
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(
                        widget::slider(0..=100, volume, Message::ChangeVolume)
                            .width(Length::Fill),
                    )
                    .push(widget::text::caption(format!("{}%", volume))),
            )
            .push(widget::text::caption(match lang {
                crate::i18n::Language::Es => "El volumen se aplica al instante.",
                crate::i18n::Language::En => "Volume applies immediately.",
                crate::i18n::Language::Fr => "Le volume s’applique immédiatement.",
            }))
            .push(
                widget::button::standard(lang.bar_hide())
                    .leading_icon(widget::icon::from_name("go-down-symbolic"))
                    .on_press(Message::ToggleShowNowPlaying(false))
                    .width(Length::Fill),
            );

        self.bar_panel(lang.bar_volume(), content.into())
    }

    fn bar_monitors_popover(&self) -> Element<'_, Message> {
        let lang = self.language;
        let mut list = widget::column::with_capacity(self.outputs.len() + 3).spacing(5);

        let all_selected = self.selected_output == "*";
        list = list.push(
            if all_selected {
                widget::button::suggested(lang.bar_all_monitors())
            } else {
                widget::button::standard(lang.bar_all_monitors())
            }
            .leading_icon(widget::icon::from_name("video-display-symbolic"))
            .on_press(Message::SelectOutput("*".into()))
            .width(Length::Fill),
        );

        for monitor in &self.outputs {
            let selected = self.selected_output == monitor.name;
            let has_override = self.config.wallpapers.contains_key(&monitor.name);
            let marker = if has_override { " •" } else { "" };
            let label = format!("{} · {}{}", monitor.name, monitor.resolution, marker);
            list = list.push(
                if selected {
                    widget::button::suggested(label)
                } else {
                    widget::button::standard(label)
                }
                .leading_icon(widget::icon::from_name(if selected {
                    "emblem-ok-symbolic"
                } else {
                    "video-display-symbolic"
                }))
                .on_press(Message::SelectOutput(monitor.name.clone()))
                .width(Length::Fill),
            );
        }

        list = list.push(widget::text::caption(lang.bar_this_monitor()));
        list = list.push(widget::text::caption(match lang {
            crate::i18n::Language::Es => {
                "El destino se usa al aplicar un fondo desde Biblioteca o Explorar."
            }
            crate::i18n::Language::En => {
                "The target is used when applying a wallpaper from Library or Explore."
            }
            crate::i18n::Language::Fr => {
                "La cible est utilisée lors de l’application depuis Bibliothèque ou Explorer."
            }
        }));

        let list: Element<'_, Message> = if self.outputs.len() > 6 {
            Element::from(
                widget::scrollable(list)
                    .height(Length::Fixed(320.0))
                    .width(Length::Fill),
            )
        } else {
            Element::from(list)
        };
        let panel = widget::container(list)
            .width(Length::Fixed(if self.window_width < 480.0 { 250.0 } else { 300.0 }))
            .padding(12)
            .class(cosmic::theme::Container::Card);
        Element::from(panel)
    }

    fn bar_details_popover(&self, has_wallpapers: bool) -> Element<'_, Message> {
        let lang = self.language;
        let target = if self.selected_output == "*" {
            lang.bar_all_monitors().to_string()
        } else {
            self.selected_output.clone()
        };
        let rotation = if self.config.rotation {
            match lang {
                crate::i18n::Language::Es => format!("Rotación activa · {} min", self.config.interval),
                crate::i18n::Language::En => format!("Rotation active · {} min", self.config.interval),
                crate::i18n::Language::Fr => format!("Rotation active · {} min", self.config.interval),
            }
        } else {
            match lang {
                crate::i18n::Language::Es => "Rotación desactivada".to_string(),
                crate::i18n::Language::En => "Rotation disabled".to_string(),
                crate::i18n::Language::Fr => "Rotation désactivée".to_string(),
            }
        };

        let prev = widget::button::icon(widget::icon::from_name("media-skip-backward-symbolic"))
            .tooltip(lang.bar_prev())
            .on_press_maybe(has_wallpapers.then_some(Message::PrevWallpaper));
        let next = widget::button::icon(widget::icon::from_name("media-skip-forward-symbolic"))
            .tooltip(lang.bar_next())
            .on_press_maybe(has_wallpapers.then_some(Message::NextWallpaper));
        let stop = widget::button::icon(widget::icon::from_name("process-stop-symbolic"))
            .class(cosmic::theme::Button::Destructive)
            .tooltip(lang.bar_stop())
            .on_press_maybe(has_wallpapers.then_some(Message::StopWallpaper(None)));

        let content = widget::column::with_capacity(8)
            .spacing(8)
            .push(widget::text::caption(format!("{}: {}", lang.bar_target_label(), target)))
            .push(widget::text::caption(rotation))
            .push(widget::text::caption(lang.bar_gpu_accel(&self.config.hwdec)))
            .push(widget::text::caption(lang.bar_wayland_tag()))
            .push(
                widget::row::with_capacity(3)
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(
                        widget::button::icon(widget::icon::from_name(if self.config.mute {
                            "audio-volume-muted-symbolic"
                        } else {
                            "audio-volume-high-symbolic"
                        }))
                        .on_press(Message::ToggleMute(!self.config.mute)),
                    )
                    .push(
                        widget::slider(0..=100, self.config.volume, Message::ChangeVolume)
                            .width(Length::Fill),
                    )
                    .push(widget::text::caption(format!("{}%", self.config.volume))),
            )
            .push(
                widget::row::with_capacity(4)
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .push(prev)
                    .push(self.bar_play_pause_icon(has_wallpapers))
                    .push(next)
                    .push(stop),
            )
            .push(
                widget::button::standard(lang.bar_hide())
                    .leading_icon(widget::icon::from_name("go-down-symbolic"))
                    .on_press(Message::ToggleShowNowPlaying(false))
                    .width(Length::Fill),
            );

        self.bar_panel(lang.bar_details(), content.into())
    }

    pub(crate) fn view_now_playing_bar(&self) -> Element<'_, Message> {
        let has_wallpapers = !self.config.wallpapers.is_empty();
        let compact = self.window_width < 720.0 || self.window_height < 600.0;
        let mini = self.window_height < 460.0;

        let (wall_title, out_label) = if let Some(curr) = &self.config.current {
            let p = std::path::Path::new(curr);
            let name = p
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| self.language.bar_active_title_default().into());
            let target = if self.selected_output == "*" {
                self.language.bar_all_monitors().to_string()
            } else {
                self.selected_output.clone()
            };
            let rot_badge = if self.config.rotation {
                if self.config.rotation_only_favorites {
                    format!(" • ♥ {}m", self.config.interval)
                } else {
                    format!(" • ⟳ {}m", self.config.interval)
                }
            } else {
                String::new()
            };
            (name, format!("{}{}", self.language.bar_display_label(&target), rot_badge))
        } else {
            (
                self.language.bar_idle_title().into(),
                self.language.bar_idle_desc().into(),
            )
        };

        let is_muted = self.config.mute;
        let volume = self.config.volume;
        let lang = self.language;
        let title_chars = if self.window_width < 420.0 { 24 } else if compact { 32 } else { 56 };

        let prev_btn = widget::button::icon(widget::icon::from_name("media-skip-backward-symbolic"))
            .tooltip(lang.bar_prev())
            .on_press_maybe(has_wallpapers.then_some(Message::PrevWallpaper));
        let next_btn = widget::button::icon(widget::icon::from_name("media-skip-forward-symbolic"))
            .tooltip(lang.bar_next())
            .on_press_maybe(has_wallpapers.then_some(Message::NextWallpaper));

        let play_pause_btn = self.bar_play_pause_icon(has_wallpapers);

        let volume_btn = if is_muted {
            widget::button::icon(widget::icon::from_name("audio-volume-muted-symbolic"))
                .tooltip(lang.bar_muted())
        } else {
            widget::button::icon(widget::icon::from_name("audio-volume-high-symbolic"))
                .tooltip(format!("{}: {}%", lang.bar_audio_active(), volume))
        }
        .on_press(Message::ToggleBarPopover(BarPopover::Volume));

        let monitor_count = self.outputs.len().max(1);
        let target_label = if self.selected_output == "*" {
            lang.bar_all_monitors().to_string()
        } else {
            self.selected_output.clone()
        };
        let monitor_btn_compact =
            widget::button::icon(widget::icon::from_name("video-display-symbolic"))
                .tooltip(format!(
                    "{} · {}",
                    lang.bar_monitors_count(monitor_count),
                    target_label
                ))
                .on_press(Message::ToggleBarPopover(BarPopover::Monitors));
        let monitor_btn_full = widget::button::standard(lang.bar_monitors_count(monitor_count))
            .leading_icon(widget::icon::from_name("video-display-symbolic"))
            .tooltip(format!("{}: {}", lang.bar_target_label(), target_label))
            .on_press(Message::ToggleBarPopover(BarPopover::Monitors));

        let menu_button = || {
            widget::button::icon(widget::icon::from_name("open-menu-symbolic"))
                .tooltip(lang.bar_details())
                .on_press(Message::ToggleBarPopover(BarPopover::Details))
        };

        let stop_btn = widget::button::icon(widget::icon::from_name("process-stop-symbolic"))
            .class(cosmic::theme::Button::Destructive)
            .tooltip(lang.bar_stop())
            .on_press_maybe(has_wallpapers.then_some(Message::StopWallpaper(None)));


        let mut left_col = widget::column::with_capacity(if mini { 1 } else { 2 })
            .spacing(2)
            .width(Length::Fill)
            .push(widget::text::body(Self::truncate_bar_title(&wall_title, title_chars)).size(14));
        if !mini {
            let caption = if compact {
                Self::truncate_bar_title(&out_label, title_chars)
            } else {
                out_label.clone()
            };
            left_col = left_col.push(widget::text::caption(caption));
        }

        let bar_content = if mini {
            widget::row::with_capacity(3)
                .spacing(4)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .push(left_col)
                .push(self.bar_play_pause_icon(has_wallpapers))
                .push(self.bar_popover(
                    menu_button().into(),
                    BarPopover::Details,
                    self.bar_details_popover(has_wallpapers),
                ))
        } else if compact {
            widget::row::with_capacity(3)
                .spacing(6)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .push(left_col)
                .push(
                    widget::row::with_capacity(4)
                        .spacing(2)
                        .align_y(Alignment::Center)
                        .push(play_pause_btn)
                        .push(self.bar_popover(
                            monitor_btn_compact.into(),
                            BarPopover::Monitors,
                            self.bar_monitors_popover(),
                        ))
                        .push(self.bar_popover(
                            menu_button().into(),
                            BarPopover::Details,
                            self.bar_details_popover(has_wallpapers),
                        )),
                )
        } else {
            let right_col = widget::column::with_capacity(2)
                .spacing(2)
                .align_x(Horizontal::Right)
                .push(widget::text::caption(lang.bar_gpu_accel(&self.config.hwdec)))
                .push(widget::text::caption(lang.bar_wayland_tag()));
            let controls = widget::row::with_capacity(10)
                .spacing(8)
                .align_y(Alignment::Center)
                .push(prev_btn)
                .push(play_pause_btn)
                .push(self.bar_popover(
                    volume_btn.into(),
                    BarPopover::Volume,
                    self.bar_volume_popover(),
                ))
                .push(self.bar_popover(
                    monitor_btn_full.into(),
                    BarPopover::Monitors,
                    self.bar_monitors_popover(),
                ))
                .push(next_btn)
                .push(
                    widget::slider(0..=100, volume, Message::ChangeVolume)
                        .width(Length::Fixed(72.0)),
                )
                .push(widget::text::caption(format!("{}%", volume)))
                .push(stop_btn)
                .push(self.bar_popover(
                    menu_button().into(),
                    BarPopover::Details,
                    self.bar_details_popover(has_wallpapers),
                ));
            widget::row::with_capacity(3)
                .spacing(16)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .push(left_col)
                .push(controls)
                .push(right_col)
        };

        Element::from(
            widget::container(bar_content)
                .padding(if compact { [6, 10] } else { [8, 14] })
                .width(Length::Fill)
                .class(cosmic::theme::Container::Card),
        )
    }
}
