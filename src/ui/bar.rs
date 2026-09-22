use crate::app::{AuraApp, Message};
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    pub(crate) fn view_now_playing_bar(&self) -> Element<'_, Message> {
        let has_wallpapers = !self.config.wallpapers.is_empty();

        let (wall_title, out_label) = if let Some(curr) = &self.config.current {
            let p = std::path::Path::new(curr);
            let name = p
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| self.language.bar_active_title_default().into());
            let out = self.config.wallpapers.keys().cloned().collect::<Vec<_>>().join(", ");
            let base_label = self.language.bar_display_label(&out);
            let rot_badge = if self.config.rotation {
                if self.config.rotation_only_favorites {
                    format!(" • ♥ {}m", self.config.interval)
                } else {
                    format!(" • ⟳ {}m", self.config.interval)
                }
            } else {
                String::new()
            };
            (name, format!("{}{}", base_label, rot_badge))
        } else {
            (self.language.bar_idle_title().into(), self.language.bar_idle_desc().into())
        };

        let is_paused = self.is_paused;
        let is_muted = self.config.mute;
        let volume = self.config.volume;
        let hwdec = self.config.hwdec.clone();
        let lang = self.language;

        Element::from(widget::responsive(move |size| {
            let width = size.width;

            // Common Previous & Next buttons with tooltips
            let prev_btn = widget::button::icon(widget::icon::from_name("media-skip-backward-symbolic"))
                .tooltip(lang.bar_prev())
                .on_press_maybe(if has_wallpapers { Some(Message::PrevWallpaper) } else { None });

            let next_btn = widget::button::icon(widget::icon::from_name("media-skip-forward-symbolic"))
                .tooltip(lang.bar_next())
                .on_press_maybe(if has_wallpapers { Some(Message::NextWallpaper) } else { None });

            if width < 680.0 {
                // Tier 3: Compact / Tiled / Narrow window (< 680px)
                // Adaptive 2-Row layout: title and status above, centered touch-friendly controls below
                let play_pause_btn = if is_paused {
                    widget::button::icon(widget::icon::from_name("media-playback-start-symbolic"))
                        .class(cosmic::theme::Button::Suggested)
                        .tooltip(lang.bar_resume())
                        .on_press_maybe(if has_wallpapers { Some(Message::TogglePause) } else { None })
                } else {
                    widget::button::icon(widget::icon::from_name("media-playback-pause-symbolic"))
                        .tooltip(lang.bar_pause())
                        .on_press_maybe(if has_wallpapers { Some(Message::TogglePause) } else { None })
                };

                let mute_btn = if is_muted {
                    widget::button::icon(widget::icon::from_name("audio-volume-muted-symbolic"))
                        .tooltip(lang.bar_muted())
                        .on_press(Message::ToggleMute(false))
                } else {
                    widget::button::icon(widget::icon::from_name("audio-volume-high-symbolic"))
                        .class(cosmic::theme::Button::Suggested)
                        .tooltip(lang.bar_audio_active())
                        .on_press(Message::ToggleMute(true))
                };

                let stop_btn = widget::button::icon(widget::icon::from_name("process-stop-symbolic"))
                    .class(cosmic::theme::Button::Destructive)
                    .tooltip(lang.bar_stop())
                    .on_press_maybe(if has_wallpapers { Some(Message::StopWallpaper(None)) } else { None });

                let top_row = widget::row::with_capacity(2)
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .push(
                        widget::column::with_capacity(2)
                            .spacing(1)
                            .width(Length::Fill)
                            .push(widget::text::body(wall_title.clone()).size(14))
                            .push(widget::text::caption(out_label.clone()))
                    )
                    .push(
                        widget::text::caption(lang.bar_gpu_accel(&hwdec))
                    );

                let mut bottom_row = widget::row::with_capacity(7)
                    .spacing(8)
                    .align_y(Alignment::Center);

                bottom_row = bottom_row
                    .push(prev_btn)
                    .push(play_pause_btn)
                    .push(next_btn)
                    .push(mute_btn);

                if !is_muted {
                    let vol_slider = widget::slider(0..=100, volume, Message::ChangeVolume)
                        .width(Length::Fixed(75.0));
                    let vol_text = widget::text::caption(format!("{}%", volume));
                    bottom_row = bottom_row.push(vol_slider).push(vol_text);
                }

                bottom_row = bottom_row.push(stop_btn);

                let bottom_centered = widget::container(bottom_row)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center);

                let col = widget::column::with_capacity(2)
                    .spacing(8)
                    .width(Length::Fill)
                    .push(top_row)
                    .push(bottom_centered);

                widget::container(col)
                    .padding([8, 14])
                    .width(Length::Fill)
                    .into()
            } else if width < 960.0 {
                // Tier 2: Medium / Standard Window (680px - 959px)
                // Single row with compact icon buttons to preserve ample room for wallpaper title
                let play_pause_btn = if is_paused {
                    widget::button::icon(widget::icon::from_name("media-playback-start-symbolic"))
                        .class(cosmic::theme::Button::Suggested)
                        .tooltip(lang.bar_resume())
                        .on_press_maybe(if has_wallpapers { Some(Message::TogglePause) } else { None })
                } else {
                    widget::button::icon(widget::icon::from_name("media-playback-pause-symbolic"))
                        .tooltip(lang.bar_pause())
                        .on_press_maybe(if has_wallpapers { Some(Message::TogglePause) } else { None })
                };

                let mute_btn = if is_muted {
                    widget::button::icon(widget::icon::from_name("audio-volume-muted-symbolic"))
                        .tooltip(lang.bar_muted())
                        .on_press(Message::ToggleMute(false))
                } else {
                    widget::button::icon(widget::icon::from_name("audio-volume-high-symbolic"))
                        .class(cosmic::theme::Button::Suggested)
                        .tooltip(lang.bar_audio_active())
                        .on_press(Message::ToggleMute(true))
                };

                let stop_btn = widget::button::icon(widget::icon::from_name("process-stop-symbolic"))
                    .class(cosmic::theme::Button::Destructive)
                    .tooltip(lang.bar_stop())
                    .on_press_maybe(if has_wallpapers { Some(Message::StopWallpaper(None)) } else { None });

                let mut center_controls = widget::row::with_capacity(7)
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(prev_btn)
                    .push(play_pause_btn)
                    .push(next_btn)
                    .push(mute_btn);

                if !is_muted {
                    let vol_slider = widget::slider(0..=100, volume, Message::ChangeVolume)
                        .width(Length::Fixed(70.0));
                    let vol_text = widget::text::caption(format!("{}%", volume));
                    center_controls = center_controls.push(vol_slider).push(vol_text);
                }

                center_controls = center_controls.push(stop_btn);

                let left_col = widget::column::with_capacity(2)
                    .spacing(2)
                    .width(Length::Fill)
                    .push(widget::text::body(wall_title.clone()).size(14))
                    .push(widget::text::caption(out_label.clone()));

                let right_col = widget::column::with_capacity(1)
                    .align_x(Horizontal::Right)
                    .push(widget::text::caption(lang.bar_gpu_accel(&hwdec)));

                let bar_row = widget::row::with_capacity(3)
                    .spacing(16)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .push(left_col)
                    .push(center_controls)
                    .push(right_col);

                widget::container(bar_row)
                    .padding([10, 16])
                    .width(Length::Fill)
                    .into()
            } else {
                // Tier 1: Wide Desktop / Fullscreen (>= 960px)
                // Spacious 3-column single row with full labeled buttons
                let play_pause_btn = if is_paused {
                    widget::button::suggested(lang.bar_resume())
                        .leading_icon(widget::icon::from_name("media-playback-start-symbolic"))
                        .on_press_maybe(if has_wallpapers { Some(Message::TogglePause) } else { None })
                } else {
                    widget::button::standard(lang.bar_pause())
                        .leading_icon(widget::icon::from_name("media-playback-pause-symbolic"))
                        .on_press_maybe(if has_wallpapers { Some(Message::TogglePause) } else { None })
                };

                let mute_btn = if is_muted {
                    widget::button::standard(lang.bar_muted())
                        .leading_icon(widget::icon::from_name("audio-volume-muted-symbolic"))
                        .on_press(Message::ToggleMute(false))
                } else {
                    widget::button::suggested(lang.bar_audio_active())
                        .leading_icon(widget::icon::from_name("audio-volume-high-symbolic"))
                        .on_press(Message::ToggleMute(true))
                };

                let stop_btn = widget::button::destructive(lang.bar_stop())
                    .leading_icon(widget::icon::from_name("process-stop-symbolic"))
                    .on_press_maybe(if has_wallpapers { Some(Message::StopWallpaper(None)) } else { None });

                let mut center_controls = widget::row::with_capacity(7)
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(prev_btn)
                    .push(play_pause_btn)
                    .push(next_btn)
                    .push(mute_btn);

                if !is_muted {
                    let vol_slider = widget::slider(0..=100, volume, Message::ChangeVolume)
                        .width(Length::Fixed(90.0));
                    let vol_text = widget::text::caption(format!("{}: {}%", lang.bar_volume(), volume));
                    center_controls = center_controls.push(vol_slider).push(vol_text);
                }

                center_controls = center_controls.push(stop_btn);

                let left_col = widget::column::with_capacity(2)
                    .spacing(2)
                    .width(Length::Fill)
                    .push(widget::text::body(wall_title.clone()).size(15))
                    .push(widget::text::caption(out_label.clone()));

                let right_col = widget::column::with_capacity(2)
                    .spacing(2)
                    .align_x(Horizontal::Right)
                    .push(widget::text::caption(lang.bar_gpu_accel(&hwdec)))
                    .push(widget::text::caption(lang.bar_wayland_tag()));

                let bar_row = widget::row::with_capacity(3)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .push(left_col)
                    .push(center_controls)
                    .push(right_col);

                widget::container(bar_row)
                    .padding([12, 18])
                    .width(Length::Fill)
                    .into()
            }
        }))
    }
}
