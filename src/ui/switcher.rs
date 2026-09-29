use crate::app::{AuraApp, Message};
use crate::config::SwitcherStyle;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    pub(crate) fn view_quick_switcher(&self) -> Element<'_, Message> {
        match self.config.switcher_style {
            SwitcherStyle::Classic => self.view_classic_switcher(),
            SwitcherStyle::Cinematic => self.view_cinematic_switcher(),
        }
    }

    pub(crate) fn view_classic_switcher(&self) -> Element<'_, Message> {
        let pool = self.switcher_pool();
        let n = pool.len();
        if n == 0 {
            let empty_text = widget::text::body(self.language.library_empty_title()).size(16);
            let empty_container = widget::container(empty_text)
                .padding(24)
                .class(cosmic::theme::Container::Card);
            let pod_mouse_area = widget::mouse_area(empty_container).on_press(Message::SwitcherNoop);
            let positioned = widget::container(pod_mouse_area)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .class(cosmic::theme::Container::Transparent);
            return widget::mouse_area(positioned)
                .on_press(Message::CloseQuickSwitcher)
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
                widget::button::image(thumb.clone())
                    .width(w)
                    .height(h)
                    .on_press(Message::SwitcherApplyIndex(item_idx))
                    .into()
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

                widget::button::custom(placeholder)
                    .on_press(Message::SwitcherApplyIndex(item_idx))
                    .into()
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

        // Prevent clicks inside the pod from triggering the outside close
        let pod_mouse_area = widget::mouse_area(floating_pod).on_press(Message::SwitcherNoop);

        let pos = self.config.switcher_position.as_str();
        let positioned = match pos {
            "bottom" => widget::container(pod_mouse_area)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Bottom)
                .padding([0, 0, 14, 0]),
            "left" => widget::container(pod_mouse_area)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Left)
                .align_y(Vertical::Center)
                .padding([0, 0, 0, 14]),
            "right" => widget::container(pod_mouse_area)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Right)
                .align_y(Vertical::Center)
                .padding([0, 14, 0, 0]),
            _ => widget::container(pod_mouse_area) // "top" default
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Top)
                .padding([14, 0, 0, 0]),
        };

        widget::mouse_area(
            positioned.class(cosmic::theme::Container::Transparent)
        )
        .on_press(Message::CloseQuickSwitcher)
        .into()
    }

    pub(crate) fn view_cinematic_switcher(&self) -> Element<'_, Message> {
        let pool = self.switcher_pool();
        let n = pool.len();
        if n == 0 {
            let empty_text = widget::text::body(self.language.library_empty_title()).size(16);
            let empty_container = widget::container(empty_text)
                .padding(24)
                .class(cosmic::theme::Container::Card);
            let pod_mouse_area = widget::mouse_area(empty_container).on_press(Message::SwitcherNoop);
            let positioned = widget::container(pod_mouse_area)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .class(cosmic::theme::Container::custom(|_theme| {
                    cosmic::iced::widget::container::Style {
                        background: Some(cosmic::iced::Background::Color(cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.38))),
                        ..Default::default()
                    }
                }));
            return widget::mouse_area(positioned)
                .on_press(Message::CloseQuickSwitcher)
                .into();
        }

        let curr_idx = self.switcher_index % n;

        // Slots configuration: (offset, width, height, x_shift, is_center)
        // Ordered from outer cards inward so the center card (0) is pushed LAST in the Stack and renders on TOP.
        let slots: Vec<(i32, f32, f32, f32, bool)> = if n >= 7 {
            vec![
                (-3, 230.0, 205.0, -540.0, false),
                (3, 230.0, 205.0, 540.0, false),
                (-2, 290.0, 260.0, -380.0, false),
                (2, 290.0, 260.0, 380.0, false),
                (-1, 350.0, 315.0, -200.0, false),
                (1, 350.0, 315.0, 200.0, false),
                (0, 420.0, 380.0, 0.0, true),
            ]
        } else if n >= 5 {
            vec![
                (-2, 290.0, 260.0, -380.0, false),
                (2, 290.0, 260.0, 380.0, false),
                (-1, 350.0, 315.0, -200.0, false),
                (1, 350.0, 315.0, 200.0, false),
                (0, 420.0, 380.0, 0.0, true),
            ]
        } else if n == 4 {
            vec![
                (-2, 290.0, 260.0, -380.0, false),
                (-1, 350.0, 315.0, -200.0, false),
                (1, 350.0, 315.0, 200.0, false),
                (0, 420.0, 380.0, 0.0, true),
            ]
        } else if n == 3 {
            vec![
                (-1, 350.0, 315.0, -200.0, false),
                (1, 350.0, 315.0, 200.0, false),
                (0, 420.0, 380.0, 0.0, true),
            ]
        } else if n == 2 {
            vec![
                (1, 350.0, 315.0, 200.0, false),
                (0, 420.0, 380.0, 0.0, true),
            ]
        } else {
            vec![(0, 420.0, 380.0, 0.0, true)]
        };

        let accent_base = cosmic::theme::active().cosmic().accent.base;
        let accent_color: cosmic::iced::Color = accent_base.into();
        let accent_rgb: [u8; 3] = [
            (accent_color.r * 255.0).round() as u8,
            (accent_color.g * 255.0).round() as u8,
            (accent_color.b * 255.0).round() as u8,
        ];

        let mut stack_children: Vec<Element<'_, Message>> = Vec::with_capacity(slots.len());

        for (offset, w, h, x_shift, is_center) in slots {
            let item_idx = ((curr_idx as i32 + offset).rem_euclid(n as i32)) as usize;
            let video = pool[item_idx];

            let card_widget: Element<'_, Message> = if let Some(cinematic_thumb) = crate::scanner::thumbs::cinematic_thumb_for_media(&video.path, is_center, Some(accent_rgb)) {
                let img = widget::image(cinematic_thumb)
                    .width(Length::Fixed(w))
                    .height(Length::Fixed(h));
                widget::mouse_area(img)
                    .on_press(Message::SwitcherApplyIndex(item_idx))
                    .into()
            } else if let Some(thumb) = &video.thumb_path {
                widget::button::image(thumb.clone())
                    .width(w)
                    .height(h)
                    .on_press(Message::SwitcherApplyIndex(item_idx))
                    .into()
            } else {
                let icon_name = if video.is_video {
                    "video-x-generic-symbolic"
                } else {
                    "image-x-generic-symbolic"
                };
                let placeholder = widget::container(widget::icon::from_name(icon_name).size(32))
                    .width(Length::Fixed(w))
                    .height(Length::Fixed(h))
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .class(cosmic::theme::Container::Card);

                widget::button::custom(placeholder)
                    .on_press(Message::SwitcherApplyIndex(item_idx))
                    .into()
            };

            let positioned_card: Element<'_, Message> = if x_shift > 0.0 {
                let r = widget::row::with_capacity(2)
                    .push(widget::Space::new().width(Length::Fixed(2.0 * x_shift)))
                    .push(card_widget);
                widget::container(r)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .into()
            } else if x_shift < 0.0 {
                let r = widget::row::with_capacity(2)
                    .push(card_widget)
                    .push(widget::Space::new().width(Length::Fixed(2.0 * x_shift.abs())));
                widget::container(r)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .into()
            } else {
                widget::container(card_widget)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .into()
            };

            stack_children.push(positioned_card);
        }

        let carousel_stack = cosmic::iced::widget::stack(stack_children)
            .width(Length::Fill)
            .height(Length::Fixed(440.0));

        // Prevent clicking on the carousel items from closing the HUD via backdrop
        let content_mouse_area = widget::mouse_area(carousel_stack).on_press(Message::SwitcherNoop);

        let centered_content = widget::container(content_mouse_area)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center);

        let backdrop = widget::container(centered_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .class(cosmic::theme::Container::custom(|_theme| {
                cosmic::iced::widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.38))),
                    ..Default::default()
                }
            }));

        widget::mouse_area(backdrop)
            .on_press(Message::CloseQuickSwitcher)
            .into()
    }
}
