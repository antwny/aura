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
            let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| self.language.bar_active_title_default().into());
            let out = self.config.wallpapers.keys().cloned().collect::<Vec<_>>().join(", ");
            (name, self.language.bar_display_label(&out))
        } else {
            (self.language.bar_idle_title().into(), self.language.bar_idle_desc().into())
        };

        let prev_btn = widget::button::icon(widget::icon::from_name("media-skip-backward-symbolic"))
            .on_press_maybe(if has_wallpapers { Some(Message::PrevWallpaper) } else { None });

        let play_pause_btn = if self.is_paused {
            widget::button::suggested(self.language.bar_resume()).on_press_maybe(if has_wallpapers { Some(Message::TogglePause) } else { None })
        } else {
            widget::button::standard(self.language.bar_pause()).on_press_maybe(if has_wallpapers { Some(Message::TogglePause) } else { None })
        };

        let next_btn = widget::button::icon(widget::icon::from_name("media-skip-forward-symbolic"))
            .on_press_maybe(if has_wallpapers { Some(Message::NextWallpaper) } else { None });

        let mute_btn = if self.config.mute {
            widget::button::standard(self.language.bar_muted())
                .leading_icon(widget::icon::from_name("audio-volume-muted-symbolic"))
                .on_press(Message::ToggleMute(false))
        } else {
            widget::button::suggested(self.language.bar_audio_active())
                .leading_icon(widget::icon::from_name("audio-volume-high-symbolic"))
                .on_press(Message::ToggleMute(true))
        };

        let stop_btn = widget::button::destructive(self.language.bar_stop())
            .leading_icon(widget::icon::from_name("process-stop-symbolic"))
            .on_press_maybe(if has_wallpapers { Some(Message::StopWallpaper(None)) } else { None });

        let mut center_controls = widget::row::with_capacity(6)
            .spacing(10)
            .align_y(Alignment::Center)
            .push(prev_btn)
            .push(play_pause_btn)
            .push(next_btn)
            .push(mute_btn);

        if !self.config.mute {
            let vol_slider = widget::slider(0..=100, self.config.volume, Message::ChangeVolume)
                .width(Length::Fixed(90.0));
            let vol_text = widget::text::caption(format!("{}: {}%", self.language.bar_volume(), self.config.volume));
            center_controls = center_controls.push(vol_slider).push(vol_text);
        }

        center_controls = center_controls.push(stop_btn);

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
            // Center: Playback Controls & Volume
            .push(center_controls)
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
}
