use crate::app::{AuraApp, Message};
use crate::online::{OnlineSource, OnlineWallpaperItem};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    pub(crate) fn view_explore(&self) -> Element<'_, Message> {
        let lang = self.language;
        let active_wallpapers = self.config.wallpapers.clone();
        let downloading_ids = self.downloading_online_ids.clone();
        let online_thumbs = self.online_thumbs.clone();
        let current_source = self.explore_source;
        let wallhaven_cat = &self.wallhaven_category;
        let wallhaven_sort = &self.wallhaven_sorting;
        let wallhaven_search = &self.wallhaven_search;
        let is_loading_more = self.explore_loading_more;

        let build_header = || {
            let bing_btn = if current_source == OnlineSource::Bing {
                widget::button::suggested(lang.explore_source_bing())
            } else {
                widget::button::standard(lang.explore_source_bing())
            }
            .leading_icon(widget::icon::from_name("image-x-generic-symbolic"))
            .on_press(Message::SelectExploreSource(OnlineSource::Bing));

            let wallhaven_btn = if current_source == OnlineSource::Wallhaven {
                widget::button::suggested(lang.explore_source_wallhaven())
            } else {
                widget::button::standard(lang.explore_source_wallhaven())
            }
            .leading_icon(widget::icon::from_name("view-grid-symbolic"))
            .on_press(Message::SelectExploreSource(OnlineSource::Wallhaven));

            let minimal_btn = if current_source == OnlineSource::Minimalistic {
                widget::button::suggested(lang.explore_source_minimalistic())
            } else {
                widget::button::standard(lang.explore_source_minimalistic())
            }
            .leading_icon(widget::icon::from_name("applications-graphics-symbolic"))
            .on_press(Message::SelectExploreSource(OnlineSource::Minimalistic));

            let reload_btn = widget::button::icon(widget::icon::from_name("view-refresh-symbolic"))
                .on_press(Message::FetchOnlineWallpapers(current_source));

            let top_bar = widget::row::with_capacity(4)
                .spacing(12)
                .align_y(Alignment::Center)
                .push(bing_btn)
                .push(wallhaven_btn)
                .push(minimal_btn)
                .push(reload_btn);

            let subtitle = match current_source {
                OnlineSource::Bing => lang.explore_featured_today(),
                OnlineSource::Wallhaven => lang.explore_recent_title(),
                OnlineSource::Minimalistic => lang.explore_minimalistic_subtitle(),
            };

            let mut header_col = widget::column::with_capacity(4)
                .spacing(8)
                .push(top_bar)
                .push(widget::text::caption(subtitle));

            if current_source == OnlineSource::Wallhaven {
                let search_input = widget::text_input::search_input(
                    lang.explore_search_placeholder(),
                    wallhaven_search,
                )
                .on_input(Message::WallhavenSearchChanged)
                .on_submit(|_| Message::SubmitWallhavenSearch)
                .width(Length::Fill);

                // Categories
                let cat_all = if wallhaven_cat == "110" {
                    widget::button::suggested(lang.explore_cat_all())
                } else {
                    widget::button::standard(lang.explore_cat_all())
                }.on_press(Message::SelectWallhavenCategory("110".into()));

                let cat_anime = if wallhaven_cat == "010" {
                    widget::button::suggested(lang.explore_cat_anime())
                } else {
                    widget::button::standard(lang.explore_cat_anime())
                }.on_press(Message::SelectWallhavenCategory("010".into()));

                let cat_gen = if wallhaven_cat == "100" {
                    widget::button::suggested(lang.explore_cat_general())
                } else {
                    widget::button::standard(lang.explore_cat_general())
                }.on_press(Message::SelectWallhavenCategory("100".into()));

                // Sorting
                let sort_top = if wallhaven_sort == "toplist" {
                    widget::button::suggested(lang.explore_sort_top())
                } else {
                    widget::button::standard(lang.explore_sort_top())
                }.on_press(Message::SelectWallhavenSorting("toplist".into()));

                let sort_hot = if wallhaven_sort == "hot" {
                    widget::button::suggested(lang.explore_sort_hot())
                } else {
                    widget::button::standard(lang.explore_sort_hot())
                }.on_press(Message::SelectWallhavenSorting("hot".into()));

                let sort_rand = if wallhaven_sort == "random" {
                    widget::button::suggested(lang.explore_sort_random())
                } else {
                    widget::button::standard(lang.explore_sort_random())
                }.on_press(Message::SelectWallhavenSorting("random".into()));

                let filters_row = widget::row::with_capacity(8)
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(cat_all)
                    .push(cat_anime)
                    .push(cat_gen)
                    .push(widget::text::caption("|"))
                    .push(sort_top)
                    .push(sort_hot)
                    .push(sort_rand);

                // Resolutions / Ratios
                let wallhaven_res = &self.wallhaven_resolution;
                let res_all = if wallhaven_res == "all" {
                    widget::button::suggested(lang.explore_res_all())
                } else {
                    widget::button::standard(lang.explore_res_all())
                }.on_press(Message::SelectWallhavenResolution("all".into()));

                let res_4k = if wallhaven_res == "4k" {
                    widget::button::suggested(lang.explore_res_4k())
                } else {
                    widget::button::standard(lang.explore_res_4k())
                }.on_press(Message::SelectWallhavenResolution("4k".into()));

                let res_2k = if wallhaven_res == "2k" {
                    widget::button::suggested(lang.explore_res_2k())
                } else {
                    widget::button::standard(lang.explore_res_2k())
                }.on_press(Message::SelectWallhavenResolution("2k".into()));

                let res_uw = if wallhaven_res == "ultrawide" {
                    widget::button::suggested(lang.explore_res_ultrawide())
                } else {
                    widget::button::standard(lang.explore_res_ultrawide())
                }.on_press(Message::SelectWallhavenResolution("ultrawide".into()));

                let res_row = widget::row::with_capacity(4)
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(res_all)
                    .push(res_4k)
                    .push(res_2k)
                    .push(res_uw);

                header_col = header_col.push(search_input).push(filters_row).push(res_row);
            }

            if current_source == OnlineSource::Minimalistic {
                let search_input = widget::text_input::search_input(
                    lang.explore_search_minimal_placeholder(),
                    &self.minimalistic_search,
                )
                .on_input(Message::MinimalisticSearchChanged)
                .width(Length::Fill);

                header_col = header_col.push(search_input);
            }

            header_col
        };

        if self.explore_loading {
            let loading_content = widget::column::with_capacity(3)
                .spacing(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title2(lang.explore_loading()))
                .push(widget::text::caption(match current_source {
                    OnlineSource::Bing => "Bing Daily Wallpaper UHD (4K)",
                    OnlineSource::Wallhaven => "Wallhaven Anime & Nature 4K",
                    OnlineSource::Minimalistic => "Minimalistic Flat Art & Nature Collection",
                }));

            let loading_view = widget::container(loading_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            return Element::from(
                widget::column::with_capacity(2)
                    .spacing(14)
                    .push(build_header())
                    .push(loading_view)
            );
        }

        if let Some(err) = &self.explore_error {
            let error_content = widget::column::with_capacity(3)
                .spacing(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title3(format!("{} {}", lang.explore_error_prefix(), err)))
                .push(
                    widget::button::suggested(lang.explore_btn_retry())
                        .leading_icon(widget::icon::from_name("view-refresh-symbolic"))
                        .on_press(Message::FetchOnlineWallpapers(current_source))
                );

            let error_view = widget::container(error_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            return Element::from(
                widget::column::with_capacity(2)
                    .spacing(14)
                    .push(build_header())
                    .push(error_view)
            );
        }

        let items: Vec<OnlineWallpaperItem> = match current_source {
            OnlineSource::Bing => self.bing_wallpapers.clone(),
            OnlineSource::Wallhaven => self.wallhaven_wallpapers.clone(),
            OnlineSource::Minimalistic => self.minimalistic_wallpapers.clone(),
        };

        if items.is_empty() {
            let empty_content = widget::column::with_capacity(3)
                .spacing(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title3(lang.explore_loading()))
                .push(
                    widget::button::suggested(lang.explore_btn_retry())
                        .leading_icon(widget::icon::from_name("view-refresh-symbolic"))
                        .on_press(Message::FetchOnlineWallpapers(current_source))
                );

            let empty_view = widget::container(empty_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            return Element::from(
                widget::column::with_capacity(2)
                    .spacing(14)
                    .push(build_header())
                    .push(empty_view)
            );
        }

        let grid = widget::responsive(move |size| {
            let card_w = 264.0f32;
            let gap = 16.0f32;
            let available_w = size.width.max(280.0);
            let cols = ((available_w + gap) / (card_w + gap)).floor().max(1.0) as usize;

            let mut cards_column = widget::column::with_capacity(items.len() / cols + 1)
                .spacing(gap)
                .width(Length::Fill);

            for chunk in items.chunks(cols) {
                let mut card_row = widget::row::with_capacity(chunk.len()).spacing(gap);
                for item in chunk {
                    let is_downloading = downloading_ids.contains(&item.id);
                    let downloaded_path = item.is_downloaded();
                    let is_active = if let Some(dp) = &downloaded_path {
                        active_wallpapers.values().any(|p| p == &dp.to_string_lossy().to_string())
                    } else {
                        false
                    };

                    let thumb_path = online_thumbs.get(&item.id).cloned().or_else(|| {
                        let p = item.local_thumb_path();
                        if p.exists() { Some(p) } else { None }
                    });

                    let mut card_content = widget::column::with_capacity(6).spacing(8).padding(12);

                    // Image preview button
                    if let Some(thumb) = &thumb_path {
                        let img_press = if let Some(dp) = &downloaded_path {
                            Message::ApplyDownloadedOnlineWallpaper(dp.clone())
                        } else {
                            Message::DownloadOnlineWallpaper {
                                item: item.clone(),
                                auto_apply: true,
                            }
                        };

                        let img_btn = widget::button::image(thumb.to_string_lossy().to_string())
                            .width(240.0)
                            .selected(is_active)
                            .on_press(img_press);
                        card_content = card_content.push(img_btn);
                    } else {
                        let placeholder = widget::container(widget::text::caption(lang.explore_loading()))
                            .width(Length::Fixed(240.0))
                            .height(Length::Fixed(135.0))
                            .align_x(Horizontal::Center)
                            .align_y(Vertical::Center);
                        card_content = card_content.push(placeholder);
                    }

                    // Title
                    let title = widget::text::body(item.title.clone()).size(14);
                    card_content = card_content.push(title);

                    // Metadata line: Resolution • Author/Date
                    let meta_text = if let Some(d) = &item.date {
                        format!("{} • {}", item.resolution, d)
                    } else if !item.author_or_copyright.is_empty() {
                        format!("{} • {}", item.resolution, item.author_or_copyright)
                    } else {
                        item.resolution.clone()
                    };
                    let meta_lbl = widget::text::caption(meta_text);
                    card_content = card_content.push(meta_lbl);

                    // Morphing Action Button with Native Symbolic Icons (NO EMOJIS)
                    let action_btn = if is_downloading {
                        widget::button::standard(lang.explore_btn_downloading())
                            .leading_icon(widget::icon::from_name("process-working-symbolic"))
                    } else if is_active {
                        widget::button::suggested(lang.explore_badge_active())
                            .leading_icon(widget::icon::from_name("emblem-ok-symbolic"))
                    } else if let Some(dp) = downloaded_path {
                        widget::button::suggested(lang.explore_btn_apply())
                            .leading_icon(widget::icon::from_name("view-fullscreen-symbolic"))
                            .on_press(Message::ApplyDownloadedOnlineWallpaper(dp))
                    } else {
                        widget::button::standard(lang.explore_btn_download())
                            .leading_icon(widget::icon::from_name("folder-download-symbolic"))
                            .on_press(Message::DownloadOnlineWallpaper {
                                item: item.clone(),
                                auto_apply: false,
                            })
                    };

                    card_content = card_content.push(action_btn);

                    let card_container = widget::container(card_content)
                        .width(Length::Fixed(card_w));

                    card_row = card_row.push(card_container);
                }
                cards_column = cards_column.push(card_row);
            }

            Element::from(cards_column)
        });

        // Pagination "Load More" Button at Bottom
        let has_more = match current_source {
            OnlineSource::Bing | OnlineSource::Wallhaven => true,
            OnlineSource::Minimalistic => {
                let total_matching = if self.minimalistic_search.trim().is_empty() {
                    self.minimalistic_all_wallpapers.len()
                } else {
                    let q = self.minimalistic_search.trim().to_lowercase();
                    self.minimalistic_all_wallpapers
                        .iter()
                        .filter(|item| {
                            item.title.to_lowercase().contains(&q)
                                || item.author_or_copyright.to_lowercase().contains(&q)
                        })
                        .count()
                };
                self.minimalistic_wallpapers.len() < total_matching
            }
        };

        let mut scroll_content = widget::column::with_capacity(2)
            .spacing(16)
            .width(Length::Fill)
            .push(grid);

        if has_more {
            let load_more_btn = if is_loading_more {
                widget::button::standard(lang.explore_btn_loading_more())
                    .leading_icon(widget::icon::from_name("process-working-symbolic"))
            } else {
                widget::button::suggested(lang.explore_btn_load_more())
                    .leading_icon(widget::icon::from_name("go-down-symbolic"))
                    .on_press(Message::LoadMoreOnlineWallpapers)
            };

            let load_more_row = widget::container(load_more_btn)
                .width(Length::Fill)
                .align_x(Horizontal::Center)
                .padding(16);

            scroll_content = scroll_content.push(load_more_row);
        }

        let scroll = widget::scrollable(scroll_content).height(Length::Fill);

        Element::from(
            widget::column::with_capacity(2)
                .spacing(14)
                .push(build_header())
                .push(scroll)
        )
    }
}
