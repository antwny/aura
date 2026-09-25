use crate::app::{AuraApp, Message};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    pub(crate) fn view_quick_switcher(&self) -> Element<'_, Message> {
        let pool = self.switcher_pool();
        let n = pool.len();
        if n == 0 {
            let empty_text = widget::text::body(self.language.library_empty_title()).size(16);
            let empty_container = widget::container(empty_text)
                .padding(24)
                .class(cosmic::theme::Container::Card);
            return widget::container(empty_container)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .class(cosmic::theme::Container::Transparent)
                .into();
        }

        let curr_idx = self.switcher_index % n;

        let is_vertical = matches!(self.config.switcher_position.as_str(), "left" | "right");

        // Cards configuration for up to 5 slots: [-2, -1, 0, +1, +2]
        // (offset, width, height, is_center)
        let slots: Vec<(i32, f32, f32, bool)> = if is_vertical {
            if n >= 5 {
                vec![
                    (-2, 120.0, 75.0, false),
                    (-1, 170.0, 106.0, false),
                    (0, 240.0, 150.0, true),
                    (1, 170.0, 106.0, false),
                    (2, 120.0, 75.0, false),
                ]
            } else if n == 4 {
                vec![
                    (-2, 120.0, 75.0, false),
                    (-1, 170.0, 106.0, false),
                    (0, 240.0, 150.0, true),
                    (1, 170.0, 106.0, false),
                ]
            } else if n == 3 {
                vec![
                    (-1, 170.0, 106.0, false),
                    (0, 240.0, 150.0, true),
                    (1, 170.0, 106.0, false),
                ]
            } else if n == 2 {
                vec![
                    (0, 240.0, 150.0, true),
                    (1, 170.0, 106.0, false),
                ]
            } else {
                vec![(0, 240.0, 150.0, true)]
            }
        } else {
            if n >= 5 {
                vec![
                    (-2, 120.0, 75.0, false),
                    (-1, 180.0, 112.0, false),
                    (0, 300.0, 188.0, true),
                    (1, 180.0, 112.0, false),
                    (2, 120.0, 75.0, false),
                ]
            } else if n == 4 {
                vec![
                    (-2, 120.0, 75.0, false),
                    (-1, 180.0, 112.0, false),
                    (0, 300.0, 188.0, true),
                    (1, 180.0, 112.0, false),
                ]
            } else if n == 3 {
                vec![
                    (-1, 180.0, 112.0, false),
                    (0, 300.0, 188.0, true),
                    (1, 180.0, 112.0, false),
                ]
            } else if n == 2 {
                vec![
                    (0, 300.0, 188.0, true),
                    (1, 180.0, 112.0, false),
                ]
            } else {
                vec![(0, 300.0, 188.0, true)]
            }
        };

        let mut elements: Vec<Element<'_, Message>> = Vec::with_capacity(slots.len());

        for (offset, w, h, is_center) in slots {
            let item_idx = ((curr_idx as i32 + offset).rem_euclid(n as i32)) as usize;
            let video = pool[item_idx];

            let card_widget: Element<'_, Message> = if let Some(thumb) = &video.thumb_path {
                let img_btn = widget::button::image(thumb.clone())
                    .width(w)
                    .height(h);

                if is_center {
                    img_btn.on_press(Message::SwitcherApply).into()
                } else {
                    img_btn.on_press(Message::SwitcherSelect(item_idx)).into()
                }
            } else {
                let icon_name = if video.is_video {
                    "video-x-generic-symbolic"
                } else {
                    "image-x-generic-symbolic"
                };
                let placeholder = widget::container(widget::icon::from_name(icon_name).size(24))
                    .width(Length::Fixed(w))
                    .height(Length::Fixed(h))
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center);

                if is_center {
                    widget::button::custom(placeholder)
                        .on_press(Message::SwitcherApply)
                        .into()
                } else {
                    widget::button::custom(placeholder)
                        .on_press(Message::SwitcherSelect(item_idx))
                        .into()
                }
            };

            if is_center {
                // Active middle wallpaper with system COSMIC accent-colored border
                let bordered_center = widget::container(card_widget)
                    .padding(3)
                    .class(cosmic::theme::Container::custom(move |theme| {
                        let accent_color: cosmic::iced::Color = theme.cosmic().accent.base.into();
                        cosmic::iced::widget::container::Style {
                            border: cosmic::iced::Border {
                                color: accent_color,
                                width: 3.0,
                                radius: 12.0.into(),
                            },
                            ..Default::default()
                        }
                    }));
                elements.push(bordered_center.into());
            } else {
                elements.push(card_widget);
            }
        }

        let floating_pod = if is_vertical {
            let mut col = widget::column::with_capacity(elements.len())
                .spacing(12)
                .align_x(Alignment::Center);
            for el in elements {
                col = col.push(el);
            }
            widget::container(col)
                .padding([18, 14])
                .class(cosmic::theme::Container::Card)
        } else {
            let mut row = widget::row::with_capacity(elements.len())
                .spacing(14)
                .align_y(Alignment::Center);
            for el in elements {
                row = row.push(el);
            }
            widget::container(row)
                .padding([14, 18])
                .class(cosmic::theme::Container::Card)
        };

        widget::container(floating_pod)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .class(cosmic::theme::Container::Transparent)
            .into()
    }
}
