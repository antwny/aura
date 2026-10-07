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
            SwitcherStyle::Honeycomb => self.view_honeycomb_switcher(),
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
}

pub(crate) fn cinematic_card_hover_dimensions(base_w: f32, base_h: f32, offset: i32, progress: f32) -> (f32, f32) {
    let (delta_w, delta_h) = match offset.abs() {
        0 => (44.0, 40.0),
        1 => (38.0, 34.0),
        2 => (32.0, 28.0),
        _ => (26.0, 24.0),
    };
    (base_w + progress * delta_w, base_h + progress * delta_h)
}

pub(crate) fn cinematic_lerp_step(current: f32, target: f32) -> (f32, bool) {
    let diff = target - current;
    if diff.abs() > 0.005 {
        (current + diff * 0.15, true)
    } else {
        (target, false)
    }
}

pub(crate) fn cinematic_hit_test(
    win_w: f32,
    win_h: f32,
    pos_x: f32,
    pos_y: f32,
    slide_offset: f32,
    scales: &[f32; 7],
    pool_len: usize,
) -> Option<i32> {
    if pool_len == 0 {
        return None;
    }
    let cx = win_w * 0.5;
    let cy = win_h * 0.5;

    let hit_candidates: &[(i32, f32, f32, f32)] = if pool_len >= 7 {
        &[
            (0, 420.0, 380.0, 0.0),
            (-1, 350.0, 315.0, -200.0),
            (1, 350.0, 315.0, 200.0),
            (-2, 290.0, 260.0, -380.0),
            (2, 290.0, 260.0, 380.0),
            (-3, 230.0, 205.0, -540.0),
            (3, 230.0, 205.0, 540.0),
        ]
    } else if pool_len >= 5 {
        &[
            (0, 420.0, 380.0, 0.0),
            (-1, 350.0, 315.0, -200.0),
            (1, 350.0, 315.0, 200.0),
            (-2, 290.0, 260.0, -380.0),
            (2, 290.0, 260.0, 380.0),
        ]
    } else if pool_len == 4 {
        &[
            (0, 420.0, 380.0, 0.0),
            (-1, 350.0, 315.0, -200.0),
            (1, 350.0, 315.0, 200.0),
            (-2, 290.0, 260.0, -380.0),
        ]
    } else if pool_len == 3 {
        &[
            (0, 420.0, 380.0, 0.0),
            (-1, 350.0, 315.0, -200.0),
            (1, 350.0, 315.0, 200.0),
        ]
    } else if pool_len == 2 {
        &[
            (0, 420.0, 380.0, 0.0),
            (1, 350.0, 315.0, 200.0),
        ]
    } else {
        &[(0, 420.0, 380.0, 0.0)]
    };

    for &(offset, base_w, base_h, x_shift) in hit_candidates {
        let slot_idx = (offset + 3).clamp(0, 6) as usize;
        let p = scales[slot_idx];
        let (w, h) = cinematic_card_hover_dimensions(base_w, base_h, offset, p);
        let card_cx = cx + x_shift + slide_offset;
        let card_cy = cy;

        let left = card_cx - w * 0.5;
        let right = card_cx + w * 0.5;
        let top = card_cy - h * 0.5;
        let bottom = card_cy + h * 0.5;

        if pos_x >= left && pos_x <= right && pos_y >= top && pos_y <= bottom {
            return Some(offset);
        }
    }
    None
}

impl AuraApp {
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

        for (offset, base_w, base_h, x_shift, is_center) in slots {
            let item_idx = ((curr_idx as i32 + offset).rem_euclid(n as i32)) as usize;
            let video = pool[item_idx];

            let slot_idx = (offset + 3).clamp(0, 6) as usize;
            let p = self.switcher_cinematic_scales[slot_idx];
            let (w, h) = cinematic_card_hover_dimensions(base_w, base_h, offset, p);

            let card_content: Element<'_, Message> = if let Some(handle) = crate::scanner::thumbs::cinematic_thumb_handle(&video.path, is_center, Some(accent_rgb)) {
                widget::image(handle)
                    .width(Length::Fixed(w))
                    .height(Length::Fixed(h))
                    .into()
            } else if let Some(cinematic_thumb) = crate::scanner::thumbs::cinematic_thumb_for_media(&video.path, is_center, Some(accent_rgb)) {
                widget::image(cinematic_thumb)
                    .width(Length::Fixed(w))
                    .height(Length::Fixed(h))
                    .into()
            } else if let Some(thumb) = &video.thumb_path {
                widget::image(thumb.clone())
                    .width(Length::Fixed(w))
                    .height(Length::Fixed(h))
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

                placeholder.into()
            };

            let card_widget: Element<'_, Message> = widget::mouse_area(card_content)
                .on_press(Message::SwitcherApplyIndex(item_idx))
                // NOTE: Do NOT add .on_enter()/.on_exit() here. Each card is wrapped in a
                // Fill×Fill container for z-stacking, so iced fires enter/exit for the entire
                // window area — not just the card image. Hover detection is done exclusively via
                // cinematic_hit_test() in window coordinates, driven by SwitcherCursorMoved.
                .interaction(cosmic::iced::mouse::Interaction::Pointer)
                .into();

            let eff_shift = x_shift + self.switcher_cinematic_slide;
            let positioned_card: Element<'_, Message> = if eff_shift > 0.0 {
                let r = widget::row::with_capacity(2)
                    .push(widget::Space::new().width(Length::Fixed(2.0 * eff_shift)))
                    .push(card_widget);
                widget::container(r)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .into()
            } else if eff_shift < 0.0 {
                let r = widget::row::with_capacity(2)
                    .push(card_widget)
                    .push(widget::Space::new().width(Length::Fixed(2.0 * eff_shift.abs())));
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
            .height(Length::Fixed(480.0));

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

    pub(crate) fn view_honeycomb_switcher(&self) -> Element<'_, Message> {
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
                        background: Some(cosmic::iced::Background::Color(cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.42))),
                        ..Default::default()
                    }
                }));
            return widget::mouse_area(positioned)
                .on_press(Message::CloseQuickSwitcher)
                .into();
        }

        let curr_idx = self.switcher_index % n;
        let accent_base = cosmic::theme::active().cosmic().accent.base;
        let accent_color: cosmic::iced::Color = accent_base.into();
        let accent_rgb: [u8; 3] = [
            (accent_color.r * 255.0).round() as u8,
            (accent_color.g * 255.0).round() as u8,
            (accent_color.b * 255.0).round() as u8,
        ];

        let responsive_honeycomb = widget::responsive(move |size| {
            SWITCHER_LOGICAL_WIDTH.store(size.width.to_bits(), std::sync::atomic::Ordering::Relaxed);
            SWITCHER_LOGICAL_HEIGHT.store(size.height.to_bits(), std::sync::atomic::Ordering::Relaxed);

            let layout = HoneycombLayout::new(size.width, size.height, n, curr_idx);

            let mut cards = Vec::new();

            for col in 0..layout.total_cols {
                let (col_cx, _) = layout.item_center(col, 0);

                // Frustum culling: skip columns completely off screen
                if col_cx + layout.w < -60.0 || col_cx - layout.w > size.width + 60.0 {
                    continue;
                }

                for row in 0..layout.rows_per_col {
                    let item_idx = col * layout.rows_per_col + row;
                    if item_idx >= n {
                        break;
                    }

                    let is_active = item_idx == curr_idx;
                    let (draw_x, draw_y) = layout.item_draw_pos(col, row);

                    let video = pool[item_idx];
                    let card_element: Element<'_, Message> = if let Some(handle) =
                        crate::scanner::thumbs::honeycomb_thumb_handle(&video.path, is_active, Some(accent_rgb))
                    {
                        widget::image(handle)
                            .width(Length::Fixed(layout.w))
                            .height(Length::Fixed(layout.h))
                            .into()
                    } else if let Some(hex_thumb) =
                        crate::scanner::thumbs::honeycomb_thumb_for_media(&video.path, is_active, Some(accent_rgb))
                    {
                        widget::image(hex_thumb)
                            .width(Length::Fixed(layout.w))
                            .height(Length::Fixed(layout.h))
                            .into()
                    } else if let Some(thumb) = &video.thumb_path {
                        widget::image(thumb.clone())
                            .width(Length::Fixed(layout.w))
                            .height(Length::Fixed(layout.h))
                            .into()
                    } else {
                        let icon_name = if video.is_video {
                            "video-x-generic-symbolic"
                        } else {
                            "image-x-generic-symbolic"
                        };
                        widget::container(widget::icon::from_name(icon_name).size(28))
                            .width(Length::Fixed(layout.w))
                            .height(Length::Fixed(layout.h))
                            .align_x(Horizontal::Center)
                            .align_y(Vertical::Center)
                            .class(cosmic::theme::Container::Card)
                            .into()
                    };

                    let pinned = cosmic::iced::widget::pin(card_element)
                        .x(draw_x)
                        .y(draw_y);

                    cards.push(Element::from(pinned));
                }
            }

            let stack = cosmic::iced::widget::stack(cards)
                .width(Length::Fill)
                .height(Length::Fill);

            widget::container(stack)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        });

        widget::container(responsive_honeycomb)
            .width(Length::Fill)
            .height(Length::Fill)
            .class(cosmic::theme::Container::custom(|_theme| {
                cosmic::iced::widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.42))),
                    ..Default::default()
                }
            }))
            .into()
    }
}

pub(crate) static SWITCHER_LOGICAL_WIDTH: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
pub(crate) static SWITCHER_LOGICAL_HEIGHT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Geometric layout calculator for the Honeycomb HUD.
/// Pointy-topped hexagons with uniform gap in all directions and exact pixel-level hit-testing.
#[derive(Debug, Clone, Copy)]
pub struct HoneycombLayout {
    pub screen_w: f32,
    #[allow(dead_code)]
    pub screen_h: f32,
    pub r: f32,
    pub inradius: f32,
    pub w: f32,
    pub h: f32,
    pub step_x: f32,
    pub step_y: f32,
    #[allow(dead_code)]
    pub gap: f32,
    pub rows_per_col: usize,
    pub total_cols: usize,
    #[allow(dead_code)]
    pub sel_col: usize,
    pub camera_x: f32,
    pub grid_start_y: f32,
}

impl HoneycombLayout {
    pub const GAP: f32 = 15.0;
    pub const ROWS_PER_COL: usize = 3;

    pub fn new(screen_w: f32, screen_h: f32, n: usize, curr_idx: usize) -> Self {
        let gap = Self::GAP;
        let rows_per_col = Self::ROWS_PER_COL;

        let avail_h = (screen_h * 0.80).clamp(380.0, 1020.0);
        // Total height spanned by 3 rows: 2 * step_y + h = 2 * (0.75 * h + sqrt(3)/2 * gap) + h = 2.5 * h + 1.7320508 * gap
        let h = ((avail_h - 1.7320508 * gap) / 2.5).clamp(150.0, 340.0);
        let r = h * 0.5;
        let w = h * 0.8660254f32; // sqrt(3)/2 * h
        let inradius = w * 0.5;

        let step_x = w + gap;
        let step_y = 1.5 * r + 0.8660254 * gap;

        let total_cols = if n > 0 { (n + rows_per_col - 1) / rows_per_col } else { 0 };
        let sel_col = if n > 0 { (curr_idx % n) / rows_per_col } else { 0 };

        let total_grid_w = if total_cols > 0 {
            (total_cols - 1) as f32 * step_x + 0.5 * step_x + w
        } else {
            w
        };
        let total_grid_h = (rows_per_col - 1) as f32 * step_y + h;

        let camera_x = if total_grid_w <= screen_w - 100.0 {
            (screen_w - total_grid_w) * 0.5
        } else {
            let sel_col_center_x = sel_col as f32 * step_x + 0.25 * step_x + (w * 0.5);
            let target_cam = (screen_w * 0.5) - sel_col_center_x;
            let max_cam = 60.0f32;
            let min_cam = screen_w - total_grid_w - 60.0f32;
            target_cam.clamp(min_cam, max_cam)
        };

        let grid_start_y = (screen_h - total_grid_h) * 0.5;

        Self {
            screen_w,
            screen_h,
            r,
            inradius,
            w,
            h,
            step_x,
            step_y,
            gap,
            rows_per_col,
            total_cols,
            sel_col,
            camera_x,
            grid_start_y,
        }
    }

    /// Center coordinate (x, y) for a given item in (col, row).
    pub fn item_center(&self, col: usize, row: usize) -> (f32, f32) {
        let row_stagger = if row % 2 != 0 { 0.5 * self.step_x } else { 0.0 };
        let cx = self.camera_x + col as f32 * self.step_x + row_stagger + (self.w * 0.5);
        let cy = self.grid_start_y + row as f32 * self.step_y + (self.h * 0.5);
        (cx, cy)
    }

    /// Top-left drawing position (x, y) for a given item in (col, row).
    pub fn item_draw_pos(&self, col: usize, row: usize) -> (f32, f32) {
        let (cx, cy) = self.item_center(col, row);
        (cx - self.w * 0.5, cy - self.h * 0.5)
    }

    /// Tests whether a point (px, py) lies inside a pointy-topped hexagon centered at (cx, cy).
    pub fn contains_point(&self, cx: f32, cy: f32, px: f32, py: f32) -> bool {
        let dx = (px - cx).abs();
        let dy = (py - cy).abs();

        dx <= self.inradius && (1.7320508 * dy + dx <= 1.7320508 * self.r)
    }

    /// Geometric hit-test to find the item index at screen coordinates (px, py).
    pub fn hit_test(&self, n: usize, px: f32, py: f32) -> Option<usize> {
        if n == 0 {
            return None;
        }

        for col in 0..self.total_cols {
            let (col_cx, _) = self.item_center(col, 0);
            if col_cx + self.w < 0.0 || col_cx - self.w > self.screen_w {
                continue;
            }

            for row in 0..self.rows_per_col {
                let item_idx = col * self.rows_per_col + row;
                if item_idx >= n {
                    break;
                }

                let (cx, cy) = self.item_center(col, row);
                if self.contains_point(cx, cy, px, py) {
                    return Some(item_idx);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_honeycomb_layout_uniform_gap() {
        let layout = HoneycombLayout::new(1366.0, 768.0, 12, 0);
        let w = layout.w;
        let r = layout.r;
        let g = layout.gap;

        // 1. Horizontal neighbors (col 0, row 0 and col 1, row 0)
        let (c0_x, c0_y) = layout.item_center(0, 0);
        let (c1_x, c1_y) = layout.item_center(1, 0);
        assert_eq!(c0_y, c1_y);
        let horiz_dist = (c1_x - c0_x).abs();
        let horiz_gap = horiz_dist - w;
        assert!((horiz_gap - g).abs() < 0.01, "Horizontal gap must match GAP exactly ({horiz_gap} vs {g})");

        // 2. Diagonal neighbors (col 0, row 0 and col 0, row 1)
        let (d1_x, d1_y) = layout.item_center(0, 1);
        let diag_dx = (d1_x - c0_x).abs();
        let diag_dy = (d1_y - c0_y).abs();
        // Perpendicular distance along the normal vector (1/2, sqrt(3)/2)
        let perp_dist = diag_dx * 0.5 + diag_dy * (0.8660254);
        let expected_dist = 1.7320508 * r + g;
        assert!((perp_dist - expected_dist).abs() < 0.05, "Perpendicular distance must match exactly ({perp_dist} vs {expected_dist})");
    }

    #[test]
    fn test_honeycomb_layout_hit_test() {
        let layout = HoneycombLayout::new(1366.0, 768.0, 6, 0);
        let (c0_x, c0_y) = layout.item_center(0, 0);

        // Center of item 0 (col 0, row 0)
        assert_eq!(layout.hit_test(6, c0_x, c0_y), Some(0));

        // Center of item 1 (col 0, row 1)
        let (c1_x, c1_y) = layout.item_center(0, 1);
        assert_eq!(layout.hit_test(6, c1_x, c1_y), Some(1));

        // Center of item 3 (col 1, row 0)
        let (c3_x, c3_y) = layout.item_center(1, 0);
        assert_eq!(layout.hit_test(6, c3_x, c3_y), Some(3));

        // Point squarely in horizontal gap between item 0 and item 3
        let horiz_gap_x = (c0_x + c3_x) * 0.5;
        assert_eq!(layout.hit_test(6, horiz_gap_x, c0_y), None, "Point in horizontal gap should hit nothing");

        // Point squarely in diagonal gap between item 0 and item 1
        let diag_gap_x = (c0_x + c1_x) * 0.5;
        let diag_gap_y = (c0_y + c1_y) * 0.5;
        assert_eq!(layout.hit_test(6, diag_gap_x, diag_gap_y), None, "Point in diagonal gap should hit nothing");

        // Point at bounding box corner (inradius, r) is NOT inside the pointy hexagon
        let corner_x = c0_x + layout.inradius;
        let corner_y = c0_y + layout.r;
        assert!(!layout.contains_point(c0_x, c0_y, corner_x, corner_y), "Bounding box corner must not be inside pointy hexagon");

        // Point far outside grid
        assert_eq!(layout.hit_test(6, 10.0, 10.0), None, "Point far outside grid should hit nothing");
    }

    #[test]
    fn test_cinematic_card_hover_dimensions() {
        // Offset 0 (currently selected center wallpaper):
        // Base: 420.0 x 380.0 -> Fully hovered: 444.0 x 402.0 (+24px, +22px)
        let (w_base, h_base) = cinematic_card_hover_dimensions(420.0, 380.0, 0, 0.0);
        assert_eq!(w_base, 420.0);
        assert_eq!(h_base, 380.0);

        let (w_half, h_half) = cinematic_card_hover_dimensions(420.0, 380.0, 0, 0.5);
        assert_eq!(w_half, 442.0);
        assert_eq!(h_half, 400.0);

        let (w_hover, h_hover) = cinematic_card_hover_dimensions(420.0, 380.0, 0, 1.0);
        assert_eq!(w_hover, 464.0);
        assert_eq!(h_hover, 420.0);

        // Offset 1 & -1:
        let (w1, h1) = cinematic_card_hover_dimensions(350.0, 315.0, 1, 1.0);
        let (wm1, hm1) = cinematic_card_hover_dimensions(350.0, 315.0, -1, 1.0);
        assert_eq!(w1, 388.0);
        assert_eq!(h1, 349.0);
        assert_eq!(wm1, 388.0);
        assert_eq!(hm1, 349.0);

        // Offset 2 & -2:
        let (w2, h2) = cinematic_card_hover_dimensions(290.0, 260.0, 2, 1.0);
        assert_eq!(w2, 322.0);
        assert_eq!(h2, 288.0);

        // Offset 3 & -3:
        let (w3, h3) = cinematic_card_hover_dimensions(230.0, 205.0, 3, 1.0);
        assert_eq!(w3, 256.0);
        assert_eq!(h3, 229.0);
    }

    #[test]
    fn test_cinematic_hit_test() {
        let scales = [0.0; 7];
        let win_w = 1920.0;
        let win_h = 1080.0;
        let cx = win_w * 0.5;
        let cy = win_h * 0.5;

        // Center card (offset 0): bounds are [cx - 210, cx + 210]
        assert_eq!(cinematic_hit_test(win_w, win_h, cx, cy, 0.0, &scales, 7), Some(0));

        // Offset 1: visible in [cx + 210, cx + 375], test at cx + 260
        assert_eq!(cinematic_hit_test(win_w, win_h, cx + 260.0, cy, 0.0, &scales, 7), Some(1));

        // Offset -1: visible in [cx - 375, cx - 210], test at cx - 260
        assert_eq!(cinematic_hit_test(win_w, win_h, cx - 260.0, cy, 0.0, &scales, 7), Some(-1));

        // Far outside cards
        assert_eq!(cinematic_hit_test(win_w, win_h, 100.0, 100.0, 0.0, &scales, 7), None);
    }

    #[test]
    fn test_cinematic_lerp_step_convergence() {
        // Forward transition (0.0 to 1.0)
        let mut curr = 0.0;
        let mut steps = 0;
        while steps < 60 {
            let (next, animating) = cinematic_lerp_step(curr, 1.0);
            curr = next;
            steps += 1;
            if !animating {
                break;
            }
        }
        assert_eq!(curr, 1.0, "Should cleanly snap to target 1.0");
        assert!(steps < 45, "Should converge within ~40 frames (~666ms at 60 FPS)");

        // Backward transition (1.0 to 0.0)
        let mut curr = 1.0;
        let mut steps = 0;
        while steps < 60 {
            let (next, animating) = cinematic_lerp_step(curr, 0.0);
            curr = next;
            steps += 1;
            if !animating {
                break;
            }
        }
        assert_eq!(curr, 0.0, "Should cleanly snap to target 0.0");
        assert!(steps < 45, "Should converge within ~40 frames");
    }
}
