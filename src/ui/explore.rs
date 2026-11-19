use crate::app::{AuraApp, Message};
use crate::online::filters::{CategoryFilter, ResolutionFilter};
use crate::online::{OnlineSource, OnlineWallpaperItem};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

fn chip_button<'a>(
    label: impl Into<std::borrow::Cow<'a, str>>,
    active: bool,
    message: Message,
) -> Element<'a, Message> {
    let button = if active {
        widget::button::suggested(label)
    } else {
        widget::button::standard(label)
    };
    Element::from(button.on_press(message))
}

/// Curated category chips. Every catalogue is queried; sources that cannot
/// honour the category are reported below the filters instead of showing
/// unrelated results.
const CATEGORY_CHIPS: [Option<CategoryFilter>; 8] = [
    None,
    Some(CategoryFilter::Anime),
    Some(CategoryFilter::Games),
    Some(CategoryFilter::Nature),
    Some(CategoryFilter::Fantasy),
    Some(CategoryFilter::SciFi),
    Some(CategoryFilter::Movies),
    Some(CategoryFilter::Abstract),
];

const RESOLUTION_CHIPS: [Option<ResolutionFilter>; 5] = [
    None,
    Some(ResolutionFilter::Hd1080),
    Some(ResolutionFilter::Qhd2K),
    Some(ResolutionFilter::Uhd4K),
    Some(ResolutionFilter::Ultrawide),
];

#[derive(Clone, Copy)]
enum OnlineErrorKind {
    Offline,
    Timeout,
    RateLimited,
    Unavailable,
    Generic,
}

/// Classifies a raw source error without exposing stack-trace-like details in
/// the UI. The full text still goes to the effects/console logs.
fn classify_online_error(raw: &str) -> OnlineErrorKind {
    let lower = raw.to_lowercase();
    if lower.contains("timed out") || lower.contains("timeout") {
        OnlineErrorKind::Timeout
    } else if lower.contains("429") || lower.contains("too many requests") {
        OnlineErrorKind::RateLimited
    } else if lower.contains("dns")
        || lower.contains("lookup")
        || lower.contains("connection refused")
        || lower.contains("failed to connect")
        || lower.contains("connect error")
        || lower.contains("network")
        || lower.contains("unreachable")
        || lower.contains("os error 101")
        || lower.contains("os error 111")
    {
        OnlineErrorKind::Offline
    } else if lower.contains(" 500")
        || lower.contains(" 502")
        || lower.contains(" 503")
        || lower.contains(" 504")
        || lower.contains("bad gateway")
        || lower.contains("service unavailable")
        || lower.contains("internal server error")
    {
        OnlineErrorKind::Unavailable
    } else {
        OnlineErrorKind::Generic
    }
}

/// Unified errors are formatted as `Source: detail`. Only the source name is
/// shown inline; the detail is intentionally dropped from the UI.
fn error_source_label(error: &str) -> &str {
    error.split(':').next().unwrap_or(error).trim()
}

impl AuraApp {
    fn explore_header(&self) -> Element<'_, Message> {
        let lang = self.language;

        let search_input = widget::text_input::search_input(
            lang.explore_unified_search_placeholder(),
            &self.unified_search,
        )
        .on_input(Message::UnifiedSearchChanged)
        .on_submit(|_| Message::SubmitUnifiedSearch)
        .width(Length::Fill);

        let mut search_row = widget::row::with_capacity(3)
            .spacing(8)
            .align_y(Alignment::Center)
            .push(search_input);

        if !self.unified_search.is_empty() {
            search_row = search_row.push(
                widget::button::icon(widget::icon::from_name("edit-clear-symbolic"))
                    .on_press(Message::ClearUnifiedSearch),
            );
        }

        search_row = search_row.push(
            widget::button::icon(widget::icon::from_name("view-refresh-symbolic"))
                .on_press(Message::FetchOnlineWallpapers(OnlineSource::Unified)),
        );

        let mut category_chips = vec![Element::from(widget::text::caption(
            lang.explore_filter_category(),
        ))];
        for category in CATEGORY_CHIPS {
            let label = match category {
                None => lang.explore_filter_all().to_string(),
                Some(category) => lang.explore_category_label(category).to_string(),
            };
            category_chips.push(chip_button(
                label,
                self.unified_category == category,
                Message::UnifiedCategoryChanged(category),
            ));
        }
        let category_row = widget::flex_row(category_chips)
            .spacing(6)
            .align_items(Alignment::Center)
            .width(Length::Fill);

        let mut resolution_chips = vec![Element::from(widget::text::caption(
            lang.explore_filter_resolution(),
        ))];
        for resolution in RESOLUTION_CHIPS {
            let label = match resolution {
                None => lang.explore_filter_all().to_string(),
                Some(resolution) => lang.explore_resolution_chip_label(resolution).to_string(),
            };
            resolution_chips.push(chip_button(
                label,
                self.unified_resolution == resolution,
                Message::UnifiedResolutionChanged(resolution),
            ));
        }
        let resolution_row = widget::flex_row(resolution_chips)
            .spacing(6)
            .align_items(Alignment::Center)
            .width(Length::Fill);

        let mut header = widget::column::with_capacity(4)
            .spacing(8)
            .push(search_row)
            .push(category_row)
            .push(resolution_row);

        if self.unified_skipped.is_empty() {
            header = header.push(widget::text::caption(lang.explore_unified_subtitle()));
        } else {
            let labels: Vec<&str> = self
                .unified_skipped
                .iter()
                .map(|source| source.label())
                .collect();
            let mut summary = labels.iter().take(3).copied().collect::<Vec<_>>().join(", ");
            if labels.len() > 3 {
                summary.push_str(&format!(" (+{})", labels.len() - 3));
            }
            header = header.push(widget::text::caption(format!(
                "{}: {}",
                lang.explore_filter_skipped(),
                summary
            )));
        }

        if !self.unified_errors.is_empty() {
            let labels: Vec<&str> = self
                .unified_errors
                .iter()
                .map(|error| error_source_label(error))
                .collect();
            let mut summary = labels.iter().take(2).copied().collect::<Vec<_>>().join(", ");
            if labels.len() > 2 {
                summary.push_str(&format!(" (+{})", labels.len() - 2));
            }
            let text = if summary.is_empty() {
                lang.explore_sources_failed(labels.len())
            } else {
                format!("{}: {}", lang.explore_sources_failed(labels.len()), summary)
            };
            header = header.push(
                widget::row::with_capacity(2)
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .push(widget::icon::from_name("dialog-warning-symbolic").size(14))
                    .push(widget::text::caption(text)),
            );
        }

        header.into()
    }

    pub(crate) fn view_explore(&self) -> Element<'_, Message> {
        let lang = self.language;
        let active_wallpapers = self.config.wallpapers.clone();
        let downloading_ids = self.downloading_online_ids.clone();
        let download_progress = self.download_progress.clone();
        let online_thumbs = self.online_thumbs.clone();
        let thumb_failures = self.online_thumb_failures.clone();
        let is_loading_more = self.explore_loading_more;

        if self.explore_loading {
            let loading_content = widget::column::with_capacity(2)
                .spacing(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title2(lang.explore_loading()))
                .push(widget::text::caption(lang.explore_unified_subtitle()));

            let loading_view = widget::container(loading_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            return Element::from(
                widget::column::with_capacity(2)
                    .spacing(14)
                    .push(self.explore_header())
                    .push(loading_view),
            );
        }

        if let Some(err) = &self.explore_error {
            let detail = match classify_online_error(err) {
                OnlineErrorKind::Offline => lang.explore_error_offline(),
                OnlineErrorKind::Timeout => lang.explore_error_timeout(),
                OnlineErrorKind::RateLimited => lang.explore_error_rate_limited(),
                OnlineErrorKind::Unavailable => lang.explore_error_unavailable(),
                OnlineErrorKind::Generic => lang.explore_error_generic(),
            };
            let error_content = widget::column::with_capacity(3)
                .spacing(10)
                .align_x(Horizontal::Center)
                .push(
                    widget::row::with_capacity(2)
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(widget::icon::from_name("dialog-warning-symbolic").size(28))
                        .push(widget::text::title3(lang.explore_error_title())),
                )
                .push(
                    widget::container(widget::text::caption(detail).width(Length::Fill))
                        .max_width(520.0),
                )
                .push(
                    widget::button::suggested(lang.explore_btn_retry())
                        .leading_icon(widget::icon::from_name("view-refresh-symbolic"))
                        .on_press(Message::FetchOnlineWallpapers(OnlineSource::Unified)),
                );

            let error_view = widget::container(error_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            return Element::from(
                widget::column::with_capacity(2)
                    .spacing(14)
                    .push(self.explore_header())
                    .push(error_view),
            );
        }

        let items: Vec<OnlineWallpaperItem> = self
            .unified_wallpapers
            .iter()
            .take(self.unified_visible)
            .cloned()
            .collect();

        if items.is_empty() {
            let query = self.unified_search.trim();
            let description = if query.is_empty() {
                lang.explore_no_results_desc().to_string()
            } else {
                lang.explore_no_results_search(query)
            };
            let mut actions = widget::row::with_capacity(2)
                .spacing(8)
                .align_y(Alignment::Center)
                .push(
                    widget::button::suggested(lang.explore_btn_retry())
                        .leading_icon(widget::icon::from_name("view-refresh-symbolic"))
                        .on_press(Message::FetchOnlineWallpapers(OnlineSource::Unified)),
                );
            if !query.is_empty() {
                actions = actions.push(
                    widget::button::standard(lang.explore_btn_clear_search())
                        .leading_icon(widget::icon::from_name("edit-clear-symbolic"))
                        .on_press(Message::ClearUnifiedSearch),
                );
            }
            let empty_content = widget::column::with_capacity(3)
                .spacing(10)
                .align_x(Horizontal::Center)
                .push(widget::text::title3(lang.explore_no_results_title()))
                .push(
                    widget::container(widget::text::caption(description).width(Length::Fill))
                        .max_width(520.0),
                )
                .push(actions);

            let empty_view = widget::container(empty_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            return Element::from(
                widget::column::with_capacity(2)
                    .spacing(14)
                    .push(self.explore_header())
                    .push(empty_view),
            );
        }

        let grid = widget::responsive(move |size| {
            let gap = 16.0f32;
            let available_w = size.width.max(280.0);
            let min_card_w = 240.0f32;
            let cols = ((available_w + gap) / (min_card_w + gap)).floor().max(1.0) as usize;
            let card_w = ((available_w - gap * (cols.saturating_sub(1)) as f32)
                / cols as f32)
                .clamp(220.0, 340.0);
            let preview_w = (card_w - 24.0).max(180.0);
            let preview_h = preview_w * 9.0 / 16.0;

            let mut cards_column = widget::column::with_capacity(items.len() / cols + 1)
                .spacing(gap)
                .width(Length::Fill)
                .align_x(Horizontal::Center);

            for chunk in items.chunks(cols) {
                let mut card_row = widget::row::with_capacity(chunk.len())
                    .spacing(gap)
                    .align_y(Alignment::Center)
                    .width(Length::Fill);
                for item in chunk {
                    let is_downloading = downloading_ids.contains(&item.id);
                    let downloaded_path = item.is_downloaded();
                    let is_active = if let Some(dp) = &downloaded_path {
                        active_wallpapers
                            .values()
                            .any(|p| p == &dp.to_string_lossy().to_string())
                    } else {
                        false
                    };

                    let thumb_path = online_thumbs.get(&item.id).cloned().or_else(|| {
                        let p = item.local_thumb_path();
                        if p.exists() {
                            Some(p)
                        } else {
                            None
                        }
                    });

                    let mut card_content = widget::column::with_capacity(6).spacing(8).padding(12);

                    if let Some(thumb) = &thumb_path {
                        let mut img_btn = widget::button::image(thumb.to_string_lossy().to_string())
                            .width(preview_w)
                            .selected(is_active);

                        if !is_downloading {
                            let img_press = if let Some(dp) = &downloaded_path {
                                Message::ApplyDownloadedOnlineWallpaper(dp.clone())
                            } else {
                                Message::DownloadOnlineWallpaper {
                                    item: item.clone(),
                                    auto_apply: true,
                                }
                            };
                            img_btn = img_btn.on_press(img_press);
                        }
                        card_content = card_content.push(img_btn);
                    } else {
                        let placeholder_text = if thumb_failures.contains(&item.id) {
                            lang.explore_thumb_unavailable()
                        } else {
                            lang.explore_thumb_loading()
                        };
                        let placeholder = widget::container(widget::text::caption(placeholder_text))
                            .width(Length::Fixed(preview_w))
                            .height(Length::Fixed(preview_h))
                            .align_x(Horizontal::Center)
                            .align_y(Vertical::Center);
                        card_content = card_content.push(placeholder);
                    }

                    let title = widget::text::body(item.title.clone()).size(14);
                    card_content = card_content.push(title);

                    let kind = if item.is_live() {
                        lang.explore_type_live()
                    } else {
                        lang.explore_type_static()
                    };
                    let resolution = item.resolution.trim();
                    let mut meta = format!("{} · {}", item.source.label(), kind);
                    if !resolution.is_empty()
                        && !resolution.eq_ignore_ascii_case("live")
                        && !resolution.eq_ignore_ascii_case("video")
                        && !resolution.eq_ignore_ascii_case("animated")
                        && !resolution.eq_ignore_ascii_case(kind)
                    {
                        meta.push_str(&format!(" · {}", resolution));
                    }
                    card_content = card_content.push(widget::text::caption(meta));

                    let action_widget: Element<'_, Message> = if is_downloading {
                        let prog = download_progress.get(&item.id);
                        let (p_bar, prog_label) = if let Some(&(downloaded, total_opt, pct)) = prog {
                            let mb_down = downloaded as f64 / (1024.0 * 1024.0);
                            let bar: Element<'_, Message> = if pct > 0.0 {
                                Element::from(
                                    widget::progress_bar::determinate_linear(pct)
                                        .width(Length::Fill),
                                )
                            } else {
                                Element::from(
                                    widget::progress_bar::indeterminate_linear()
                                        .width(Length::Fill),
                                )
                            };
                            let label = if let Some(total) = total_opt {
                                let mb_tot = total as f64 / (1024.0 * 1024.0);
                                format!(
                                    "{:.0}% · {:.1} / {:.1} MB",
                                    pct * 100.0,
                                    mb_down,
                                    mb_tot
                                )
                            } else {
                                format!("{:.1} MB", mb_down)
                            };
                            (bar, label)
                        } else {
                            (
                                Element::from(
                                    widget::progress_bar::indeterminate_linear()
                                        .width(Length::Fill),
                                ),
                                lang.explore_btn_downloading().to_string(),
                            )
                        };

                        let cancel_btn = widget::tooltip(
                            widget::button::icon(widget::icon::from_name("process-stop-symbolic"))
                                .on_press(Message::CancelOnlineDownload(item.id.clone())),
                            widget::text::caption(lang.explore_btn_cancel()),
                            widget::tooltip::Position::Top,
                        );

                        let status_btn = widget::button::standard(prog_label)
                            .leading_icon(widget::icon::from_name("process-working-symbolic"))
                            .width(Length::Fill);

                        let btn_row = widget::row::with_capacity(2)
                            .spacing(6)
                            .align_y(Alignment::Center)
                            .push(status_btn)
                            .push(cancel_btn);

                        Element::from(
                            widget::column::with_capacity(2)
                                .spacing(6)
                                .width(Length::Fill)
                                .push(p_bar)
                                .push(btn_row),
                        )
                    } else if is_active {
                        Element::from(
                            widget::button::suggested(lang.explore_badge_active())
                                .leading_icon(widget::icon::from_name("emblem-ok-symbolic"))
                                .width(Length::Fill),
                        )
                    } else if let Some(dp) = downloaded_path {
                        Element::from(
                            widget::button::suggested(lang.explore_btn_apply())
                                .leading_icon(widget::icon::from_name("view-fullscreen-symbolic"))
                                .on_press(Message::ApplyDownloadedOnlineWallpaper(dp))
                                .width(Length::Fill),
                        )
                    } else {
                        Element::from(
                            widget::button::standard(lang.explore_btn_download())
                                .leading_icon(widget::icon::from_name("folder-download-symbolic"))
                                .on_press(Message::DownloadOnlineWallpaper {
                                    item: item.clone(),
                                    auto_apply: false,
                                })
                                .width(Length::Fill),
                        )
                    };

                    card_content = card_content.push(action_widget);

                    let card_container = widget::container(card_content)
                        .width(Length::Fixed(card_w));

                    card_row = card_row.push(card_container);
                }
                cards_column = cards_column.push(card_row);
            }

            Element::from(cards_column)
        });

        let mut scroll_content = widget::column::with_capacity(2)
            .spacing(16)
            .width(Length::Fill)
            .push(grid);

        if self.unified_visible < self.unified_wallpapers.len() || self.unified_has_more {
            let load_more_btn = if is_loading_more {
                widget::button::standard(lang.explore_btn_loading_more())
                    .leading_icon(widget::icon::from_name("process-working-symbolic"))
            } else {
                widget::button::suggested(lang.explore_btn_load_more())
                    .leading_icon(widget::icon::from_name("go-down-symbolic"))
                    .on_press(Message::LoadMoreOnlineWallpapers)
            };

            scroll_content = scroll_content.push(
                widget::container(load_more_btn)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center)
                    .padding(16),
            );
        }

        let scroll = widget::scrollable(scroll_content)
            .id(self.explore_scroll_id.clone())
            .height(Length::Fill);

        Element::from(
            widget::column::with_capacity(2)
                .spacing(14)
                .push(self.explore_header())
                .push(scroll),
        )
    }
}
