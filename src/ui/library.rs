use crate::app::{AuraApp, LibraryFilter, LibrarySort, Message};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    pub(crate) fn view_library(&self) -> Element<'_, Message> {
        let search_input = widget::text_input::search_input(self.language.library_search_placeholder(), &self.search_query)
            .on_input(Message::SearchChanged)
            .width(Length::Fill);

        let (sort_icon, sort_label) = match self.library_sort {
            LibrarySort::Newest => ("view-sort-descending-symbolic", self.language.library_sort_newest()),
            LibrarySort::Oldest => ("view-sort-ascending-symbolic", self.language.library_sort_oldest()),
        };

        let sort_btn = widget::button::standard(sort_label)
            .leading_icon(widget::icon::from_name(sort_icon))
            .on_press(Message::ToggleLibrarySort);

        let search_row = widget::row::with_capacity(2)
            .spacing(10)
            .align_y(Alignment::Center)
            .push(search_input)
            .push(sort_btn);

        // Quick category filter counts (precalculated in O(1))
        let (count_all, count_live, count_static, count_downloaded, count_favorites) = self.category_counts;

        let all_btn = if self.library_filter == LibraryFilter::All {
            widget::button::suggested(format!("{} ({})", self.language.library_filter_all(), count_all))
        } else {
            widget::button::standard(format!("{} ({})", self.language.library_filter_all(), count_all))
        }
        .leading_icon(widget::icon::from_name("view-grid-symbolic"))
        .on_press(Message::SelectLibraryFilter(LibraryFilter::All));

        let live_btn = if self.library_filter == LibraryFilter::Live {
            widget::button::suggested(format!("{} ({})", self.language.library_filter_live(), count_live))
        } else {
            widget::button::standard(format!("{} ({})", self.language.library_filter_live(), count_live))
        }
        .leading_icon(widget::icon::from_name("video-x-generic-symbolic"))
        .on_press(Message::SelectLibraryFilter(LibraryFilter::Live));

        let static_btn = if self.library_filter == LibraryFilter::Static {
            widget::button::suggested(format!("{} ({})", self.language.library_filter_static(), count_static))
        } else {
            widget::button::standard(format!("{} ({})", self.language.library_filter_static(), count_static))
        }
        .leading_icon(widget::icon::from_name("image-x-generic-symbolic"))
        .on_press(Message::SelectLibraryFilter(LibraryFilter::Static));

        let downloaded_btn = if self.library_filter == LibraryFilter::Downloaded {
            widget::button::suggested(format!("{} ({})", self.language.library_filter_downloaded(), count_downloaded))
        } else {
            widget::button::standard(format!("{} ({})", self.language.library_filter_downloaded(), count_downloaded))
        }
        .leading_icon(widget::icon::from_name("folder-download-symbolic"))
        .on_press(Message::SelectLibraryFilter(LibraryFilter::Downloaded));

        let favorites_btn = if self.library_filter == LibraryFilter::Favorites {
            widget::button::suggested(format!("{} ({})", self.language.library_filter_favorites(), count_favorites))
        } else {
            widget::button::standard(format!("{} ({})", self.language.library_filter_favorites(), count_favorites))
        }
        .leading_icon(widget::icon::from_name("emblem-favorite-symbolic"))
        .on_press(Message::SelectLibraryFilter(LibraryFilter::Favorites));

        let open_folder_btn = widget::button::standard(self.language.open_in_file_manager())
            .leading_icon(widget::icon::from_name("folder-open-symbolic"))
            .on_press(Message::OpenWallpapersFolder);

        let filter_bar = widget::flex_row(vec![
            all_btn.into(),
            live_btn.into(),
            static_btn.into(),
            downloaded_btn.into(),
            favorites_btn.into(),
            open_folder_btn.into(),
        ])
        .spacing(8)
        .align_items(Alignment::Center)
        .width(Length::Fill);

        let mut top_section = widget::column::with_capacity(4)
            .spacing(10)
            .push(search_row)
            .push(filter_bar);

        // Favorites quick-rotation banner when browsing Favorites filter tab
        if self.library_filter == LibraryFilter::Favorites && count_favorites > 0 {
            let is_fav_rotation = self.config.rotation && self.config.rotation_only_favorites;
            let heart_icon = widget::icon::from_name("emblem-favorite-symbolic").size(24);

            let banner_info = if is_fav_rotation {
                let is_random = self.config.order == "random";
                widget::column::with_capacity(2)
                    .spacing(2)
                    .push(widget::text::title3(self.language.library_favs_rotation_title()))
                    .push(widget::text::caption(self.language.library_favs_banner_desc_active(count_favorites, self.config.interval, is_random)))
            } else {
                widget::column::with_capacity(2)
                    .spacing(2)
                    .push(widget::text::title3(self.language.library_favs_rotation_title()))
                    .push(widget::text::caption(self.language.library_favs_banner_desc_idle(count_favorites)))
            };

            let mut banner_actions = widget::row::with_capacity(2)
                .spacing(8)
                .align_y(Alignment::Center);

            if is_fav_rotation {
                banner_actions = banner_actions.push(
                    widget::button::standard(self.language.library_favs_stop_rotation())
                        .leading_icon(widget::icon::from_name("media-playback-pause-symbolic"))
                        .on_press(Message::ToggleRotation(false))
                );
            } else {
                banner_actions = banner_actions.push(
                    widget::button::suggested(self.language.library_favs_start_rotation())
                        .leading_icon(widget::icon::from_name("media-playlist-repeat-symbolic"))
                        .on_press(Message::StartFavoritesRotation)
                );
            }

            banner_actions = banner_actions.push(
                widget::button::icon(widget::icon::from_name("preferences-system-symbolic"))
                    .tooltip(self.language.library_favs_rotation_settings())
                    .on_press(Message::NavigateToPage(crate::app::Page::Settings))
            );

            let banner_row: Element<'_, Message> = if self.window_width < 620.0 {
                widget::column::with_capacity(3)
                    .spacing(8)
                    .width(Length::Fill)
                    .push(
                        widget::row::with_capacity(2)
                            .spacing(8)
                            .align_y(Alignment::Center)
                            .push(heart_icon)
                            .push(banner_info),
                    )
                    .push(banner_actions)
                    .into()
            } else {
                widget::row::with_capacity(3)
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .push(heart_icon)
                    .push(widget::container(banner_info).width(Length::Fill))
                    .push(banner_actions)
                    .into()
            };

            let banner_box = widget::container(banner_row)
                .padding(12)
                .width(Length::Fill)
                .class(cosmic::theme::Container::Card);

            top_section = top_section.push(banner_box);
        }

        // Target display selector chips
        if self.outputs.len() > 1 {
            let mut out_buttons: Vec<Element<'_, Message>> = Vec::with_capacity(self.outputs.len() + 1);
            out_buttons.push(widget::text::body(self.language.library_target_output_label()).into());

            let all_sel = self.selected_output == "*";
            let all_chip = if all_sel {
                widget::button::suggested(self.language.monitors_all_displays())
            } else {
                widget::button::standard(self.language.monitors_all_displays())
            }
            .leading_icon(widget::icon::from_name("video-display-symbolic"))
            .on_press(Message::SelectOutput("*".into()));
            out_buttons.push(all_chip.into());

            for out in &self.outputs {
                let is_sel = self.selected_output == out.name;
                let chip = if is_sel {
                    widget::button::suggested(&out.name)
                } else {
                    widget::button::standard(&out.name)
                }
                .leading_icon(widget::icon::from_name("video-display-symbolic"))
                .on_press(Message::SelectOutput(out.name.clone()));
                out_buttons.push(chip.into());
            }

            top_section = top_section.push(
                widget::flex_row(out_buttons)
                    .spacing(8)
                    .align_items(Alignment::Center)
                    .width(Length::Fill),
            );
        }

        if self.videos.is_empty() {
            let empty_msg = widget::column::with_capacity(4)
                .spacing(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title2(self.language.library_empty_title()))
                .push(widget::text::body(self.language.library_empty_desc()))
                .push(
                    widget::flex_row(vec![
                        widget::button::suggested(self.language.library_btn_add_video())
                            .leading_icon(widget::icon::from_name("list-add-symbolic"))
                            .on_press(Message::PickVideoFile)
                            .into(),
                        widget::button::standard(self.language.library_btn_add_folder())
                            .leading_icon(widget::icon::from_name("folder-symbolic"))
                            .on_press(Message::PickFolder)
                            .into(),
                    ])
                    .spacing(12)
                    .width(Length::Fill)
                );

            return Element::from(
                widget::container(empty_msg)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
            );
        }

        let search_trimmed = self.search_query.trim();
        let query_lower = if search_trimmed.is_empty() {
            None
        } else {
            Some(search_trimmed.to_lowercase())
        };

        let filtered_videos: Vec<_> = self.videos.iter().filter(|v| {
            let match_type = match self.library_filter {
                LibraryFilter::All => true,
                LibraryFilter::Live => v.is_video,
                LibraryFilter::Static => !v.is_video,
                LibraryFilter::Downloaded => v.is_downloaded,
                LibraryFilter::Favorites => self.favorites_set.contains(v.path.to_string_lossy().as_ref()),
            };

            if !match_type {
                return false;
            }

            if let Some(ref q) = query_lower {
                v.name_lower.contains(q)
            } else {
                true
            }
        }).collect();

        if filtered_videos.is_empty() {
            let empty_content = if self.library_filter == LibraryFilter::Favorites {
                widget::column::with_capacity(4)
                    .spacing(10)
                    .align_x(Horizontal::Center)
                    .push(widget::icon::from_name("emblem-favorite-symbolic").size(48))
                    .push(widget::text::title3(self.language.library_favorites_empty()))
                    .push(widget::text::caption(self.language.library_favorites_empty_desc()))
                    .push(
                        widget::button::standard(self.language.library_filter_all())
                            .leading_icon(widget::icon::from_name("view-grid-symbolic"))
                            .on_press(Message::SelectLibraryFilter(LibraryFilter::All))
                    )
            } else {
                widget::column::with_capacity(2)
                    .spacing(8)
                    .align_x(Horizontal::Center)
                    .push(widget::text::title3(self.language.library_filter_empty()))
                    .push(
                        widget::button::standard(self.language.library_filter_all())
                            .leading_icon(widget::icon::from_name("view-grid-symbolic"))
                            .on_press(Message::SelectLibraryFilter(LibraryFilter::All))
                    )
            };

            let empty_filter_msg = widget::container(empty_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            return Element::from(
                widget::column::with_capacity(2)
                    .spacing(14)
                    .push(top_section)
                    .push(empty_filter_msg)
            );
        }

        // Highlight only the wallpaper explicitly assigned to the current
        // target: "*" means the global default, a monitor name means its
        // override. Falling back to the wildcard is shown in Monitors instead
        // of making a global wallpaper look pinned to every display.
        let active_path = self
            .config
            .wallpapers
            .get(&self.selected_output)
            .cloned();
        let mut active_paths: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(1);
        if let Some(path) = active_path {
            active_paths.insert(path);
        }

        let selected_output = self.selected_output.clone();
        let custom_videos = self.config.custom_videos.clone();
        let favorites_set = self.favorites_set.clone();
        let total_count = filtered_videos.len();
        let display_limit = self.library_limit.min(total_count);
        let has_more = total_count > display_limit;
        let displayed_videos: Vec<_> = filtered_videos.into_iter().take(display_limit).collect();
        let load_more_label = self.language.library_btn_load_more();

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

            let mut cards_column = widget::column::with_capacity(displayed_videos.len() / cols + 2)
                .spacing(gap)
                .width(Length::Fill)
                .align_x(Horizontal::Center);

            for chunk in displayed_videos.chunks(cols) {
                let mut card_row = widget::row::with_capacity(chunk.len())
                    .spacing(gap)
                    .align_y(Alignment::Center);

                for video in chunk {
                    let path_str = video.path.to_string_lossy().to_string();
                    let is_active = active_paths.contains(path_str.as_str());

                    let mut card_content = widget::column::with_capacity(5).spacing(8).padding(12);

                    if let Some(thumb) = &video.thumb_path {
                        let mut img_btn = widget::button::image(thumb.clone())
                            .width(preview_w)
                            .selected(is_active);
                        if !is_active {
                            img_btn = img_btn.on_press(Message::ApplyWallpaper {
                                video_path: video.path.clone(),
                                output: selected_output.clone(),
                            });
                        }
                        card_content = card_content.push(img_btn);
                    } else {
                        let placeholder = widget::container(widget::text::body(self.language.library_extracting_frame()))
                            .width(Length::Fixed(preview_w))
                            .height(Length::Fixed(preview_h))
                            .align_x(Horizontal::Center)
                            .align_y(Vertical::Center);
                        card_content = card_content.push(placeholder);
                    }

                    let title = widget::text::body(&video.name).size(14);
                    let size_lbl = widget::text::caption(&video.size_formatted);
                    card_content = card_content.push(title).push(size_lbl);

                    let apply_btn = if is_active {
                        widget::button::suggested(self.language.library_active())
                            .leading_icon(widget::icon::from_name("emblem-ok-symbolic"))
                    } else {
                        widget::button::standard(self.language.library_apply())
                            .leading_icon(widget::icon::from_name("view-fullscreen-symbolic"))
                            .on_press(Message::ApplyWallpaper {
                                video_path: video.path.clone(),
                                output: selected_output.clone(),
                            })
                    };

                    let is_fav = favorites_set.contains(path_str.as_str());
                    let fav_tooltip = if is_fav {
                        self.language.library_fav_remove()
                    } else {
                        self.language.library_fav_add()
                    };

                    let mut fav_btn = widget::button::icon(widget::icon::from_name("emblem-favorite-symbolic"))
                        .tooltip(fav_tooltip)
                        .on_press(Message::ToggleFavorite(video.path.clone()));

                    if is_fav {
                        fav_btn = fav_btn.class(cosmic::theme::Button::Suggested);
                    }

                    let mut action_row = widget::row::with_capacity(3)
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(apply_btn)
                        .push(fav_btn);

                    // Safe delete vs remove: distinguish online download vs user file
                    if video.is_downloaded {
                        action_row = action_row.push(
                            widget::button::icon(widget::icon::from_name("user-trash-symbolic"))
                                .on_press(Message::DeleteDownloadedWallpaper(video.path.clone()))
                        );
                    } else if custom_videos.contains(&path_str) {
                        action_row = action_row.push(
                            widget::button::icon(widget::icon::from_name("list-remove-symbolic"))
                                .on_press(Message::RemoveWallpaperFromLibrary(video.path.clone()))
                        );
                    }

                    card_content = card_content.push(action_row);

                    let card_container = widget::container(card_content)
                        .width(Length::Fixed(card_w));

                    card_row = card_row.push(card_container);
                }
                cards_column = cards_column.push(card_row);
            }

            if has_more {
                let load_more_btn = widget::button::suggested(load_more_label)
                    .leading_icon(widget::icon::from_name("go-down-symbolic"))
                    .on_press(Message::LoadMoreLibraryWallpapers);

                let load_more_container = widget::container(load_more_btn)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center)
                    .padding(16);

                cards_column = cards_column.push(load_more_container);
            }

            cards_column.into()
        });

        let main_layout = widget::column::with_capacity(2)
            .spacing(14)
            .push(top_section)
            .push(
                widget::scrollable(grid)
                    .id(self.library_scroll_id.clone())
                    .height(Length::Fill),
            );

        Element::from(main_layout)
    }
}
