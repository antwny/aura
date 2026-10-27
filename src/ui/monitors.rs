use crate::app::{AuraApp, Message};
use crate::scanner::IMAGE_EXTENSIONS;
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    pub(crate) fn view_monitors(&self) -> Element<'_, Message> {
        let compact = self.window_width < 700.0;
        let mut col = widget::column::with_capacity(self.outputs.len() + 3)
            .spacing(if compact { 12 } else { 18 })
            .width(Length::Fill);

        col = col.push(widget::text::title2(self.language.monitors_title()));
        col = col.push(widget::text::body(self.language.monitors_desc()));

        // Horizontal Canvas of Virtual Monitors
        let mut monitors_canvas = widget::row::with_capacity(self.outputs.len() + 1)
            .spacing(24)
            .align_y(Alignment::End);
        let mut monitors_list = widget::column::with_capacity(self.outputs.len())
            .spacing(12)
            .width(Length::Fill);

        for monitor in &self.outputs {
            let active_wall = self.config.wallpapers.get(&monitor.name)
                .or_else(|| self.config.wallpapers.get("*"))
                .or_else(|| self.config.current.as_ref());
            let active_scaling = self.config.scaling.get(&monitor.name)
                .or_else(|| self.config.scaling.get("*"))
                .map(|s| s.as_str())
                .unwrap_or("fit");

            let screen_w = if self.window_width < 420.0 {
                (self.window_width - 100.0).clamp(180.0, 260.0)
            } else {
                260.0
            };
            let is_target = self.selected_output == monitor.name;

            // Screen frame representation with image & video thumbnail support
            let screen_display: Element<'_, Message> = if let Some(wall_path) = active_wall {
                let p = std::path::Path::new(wall_path);
                let is_image = p.extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false);

                let display_thumb = if is_image && p.exists() {
                    Some(p.to_path_buf())
                } else {
                    let thumb = crate::scanner::thumbs::thumb_path_for_video(p);
                    if thumb.exists() {
                        Some(thumb)
                    } else if p.exists() && is_image {
                        Some(p.to_path_buf())
                    } else {
                        None
                    }
                };

                if let Some(thumb_path) = display_thumb {
                    Element::from(
                        widget::button::image(thumb_path.to_string_lossy().to_string())
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
                let idle_label = match self.language {
                    crate::i18n::Language::Es => {
                        format!(
                            "Pantalla {}\n({})",
                            monitor.name,
                            self.desktop_environment().label()
                        )
                    }
                    crate::i18n::Language::En => {
                        format!(
                            "Display {}\n({})",
                            monitor.name,
                            self.desktop_environment().label()
                        )
                    }
                    crate::i18n::Language::Fr => {
                        format!(
                            "Écran {}\n({})",
                            monitor.name,
                            self.desktop_environment().label()
                        )
                    }
                };
                Element::from(
                    widget::button::standard(idle_label)
                        .width(screen_w)
                )
            };

            let target_btn = if is_target {
                widget::button::suggested(self.language.monitors_target_active())
                    .leading_icon(widget::icon::from_name("emblem-ok-symbolic"))
            } else {
                widget::button::standard(self.language.monitors_apply_to_this())
                    .leading_icon(widget::icon::from_name("video-display-symbolic"))
            }
            .on_press(Message::SelectOutput(monitor.name.clone()));

            let monitor_card = widget::column::with_capacity(7)
                .spacing(10)
                .padding(if compact { 10 } else { 14 })
                .width(if compact { Length::Fill } else { Length::Shrink })
                .align_x(Horizontal::Center)
                .push(widget::text::title3(format!("{} • {}", monitor.name, monitor.resolution)))
                .push(screen_display)
                .push(target_btn)
                .push(
                    widget::flex_row(vec![
                        if active_scaling == "fit" {
                            widget::button::suggested(self.language.scaling_fit())
                        } else {
                            widget::button::standard(self.language.scaling_fit())
                        }
                        .on_press(Message::SelectScaling { output: monitor.name.clone(), scaling: "fit".into() })
                        .into(),
                        if active_scaling == "fill" {
                            widget::button::suggested(self.language.scaling_fill())
                        } else {
                            widget::button::standard(self.language.scaling_fill())
                        }
                        .on_press(Message::SelectScaling { output: monitor.name.clone(), scaling: "fill".into() })
                        .into(),
                        if active_scaling == "stretch" {
                            widget::button::suggested(self.language.scaling_stretch())
                        } else {
                            widget::button::standard(self.language.scaling_stretch())
                        }
                        .on_press(Message::SelectScaling { output: monitor.name.clone(), scaling: "stretch".into() })
                        .into(),
                        widget::button::destructive(self.language.monitors_stop())
                            .leading_icon(widget::icon::from_name("process-stop-symbolic"))
                            .on_press(Message::StopWallpaper(Some(monitor.name.clone())))
                            .into(),
                    ])
                    .spacing(8)
                    .align_items(Alignment::Center)
                    .width(Length::Fill)
                );

            let card = widget::container(monitor_card).width(if compact {
                Length::Fill
            } else {
                Length::Shrink
            });
            if compact {
                monitors_list = monitors_list.push(card);
            } else {
                monitors_canvas = monitors_canvas.push(card);
            }
        }

        if compact {
            col = col.push(monitors_list);
        } else {
            col = col.push(widget::scrollable(monitors_canvas));
        }

        Element::from(widget::scrollable(col).height(Length::Fill))
    }
}
