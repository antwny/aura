use crate::app::{AuraApp, LibraryFilter, Message};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    pub(crate) fn view_library(&self) -> Element<'_, Message> {
        let search_bar = widget::text_input::search_input(self.language.library_search_placeholder(), &self.search_query)
            .on_input(Message::SearchChanged)
            .width(Length::Fill);

        // Quick category filter counts
        let mut count_all = 0;
        let mut count_live = 0;
        let mut count_static = 0;
        let mut count_downloaded = 0;

        for v in &self.videos {
            count_all += 1;
            if v.is_video {
                count_live += 1;
            } else {
                count_static += 1;
            }
            if v.is_downloaded {
                count_downloaded += 1;
            }
        }

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

        let open_folder_btn = widget::button::standard(self.language.open_in_file_manager())
            .leading_icon(widget::icon::from_name("folder-open-symbolic"))
            .on_press(Message::OpenWallpapersFolder);

        let filter_bar = widget::row::with_capacity(5)
            .spacing(8)
            .align_y(Alignment::Center)
            .push(all_btn)
            .push(live_btn)
            .push(static_btn)
            .push(downloaded_btn)
            .push(open_folder_btn);

        let mut top_section = widget::column::with_capacity(3)
            .spacing(10)
            .push(search_bar)
            .push(filter_bar);

        // Target display selector chips
        if self.outputs.len() > 1 {
            let mut out_row = widget::row::with_capacity(self.outputs.len() + 2)
                .spacing(8)
                .align_y(Alignment::Center);

            out_row = out_row.push(widget::text::body(self.language.library_target_output_label()));

            let all_sel = self.selected_output == "*";
            let all_chip = if all_sel {
                widget::button::suggested(self.language.monitors_all_displays())
            } else {
                widget::button::standard(self.language.monitors_all_displays())
            }
            .leading_icon(widget::icon::from_name("video-display-symbolic"))
            .on_press(Message::SelectOutput("*".into()));
            out_row = out_row.push(all_chip);

            for out in &self.outputs {
                let is_sel = self.selected_output == out.name;
                let chip = if is_sel {
                    widget::button::suggested(&out.name)
                } else {
                    widget::button::standard(&out.name)
                }
                .leading_icon(widget::icon::from_name("video-display-symbolic"))
                .on_press(Message::SelectOutput(out.name.clone()));
                out_row = out_row.push(chip);
            }

            top_section = top_section.push(out_row);
        }

        if self.videos.is_empty() {
            let empty_msg = widget::column::with_capacity(4)
                .spacing(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title2(self.language.library_empty_title()))
                .push(widget::text::body(self.language.library_empty_desc()))
                .push(
                    widget::row::with_capacity(2)
                        .spacing(12)
                        .push(
                            widget::button::suggested(self.language.library_btn_add_video())
                                .leading_icon(widget::icon::from_name("list-add-symbolic"))
                                .on_press(Message::PickVideoFile)
                        )
                        .push(
                            widget::button::standard(self.language.library_btn_add_folder())
                                .leading_icon(widget::icon::from_name("folder-symbolic"))
                                .on_press(Message::PickFolder)
                        )
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
            let empty_filter_msg = widget::container(
                widget::column::with_capacity(2)
                    .spacing(8)
                    .align_x(Horizontal::Center)
                    .push(widget::text::title3(self.language.library_filter_empty()))
                    .push(
                        widget::button::standard(self.language.library_filter_all())
                            .leading_icon(widget::icon::from_name("view-grid-symbolic"))
                            .on_press(Message::SelectLibraryFilter(LibraryFilter::All))
                    )
            )
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

        let mut active_paths = std::collections::HashSet::with_capacity(self.config.wallpapers.len() + 1);
        for path in self.config.wallpapers.values() {
            active_paths.insert(path.as_str());
        }
        if let Some(ref curr) = self.config.current {
            active_paths.insert(curr.as_str());
        }

        let selected_output = self.selected_output.clone();

        let custom_videos = self.config.custom_videos.clone();

        let grid = widget::responsive(move |size| {
            let card_w = 264.0f32;
            let gap = 16.0f32;
            let available_w = size.width.max(280.0);
            let cols = ((available_w + gap) / (card_w + gap)).floor().max(1.0) as usize;

            let mut cards_column = widget::column::with_capacity(filtered_videos.len() / cols + 1)
                .spacing(gap)
                .width(Length::Fill);

            for chunk in filtered_videos.chunks(cols) {
                let mut card_row = widget::row::with_capacity(chunk.len()).spacing(gap);
                for video in chunk {
                    let path_str = video.path.to_string_lossy().to_string();
                    let is_active = active_paths.contains(path_str.as_str());

                    let mut card_content = widget::column::with_capacity(5).spacing(8).padding(12);

                    if let Some(thumb) = &video.thumb_path {
                        let img_btn = widget::button::image(thumb.clone())
                            .width(240.0)
                            .selected(is_active)
                            .on_press(Message::ApplyWallpaper {
                                video_path: video.path.clone(),
                                output: selected_output.clone(),
                            });
                        card_content = card_content.push(img_btn);
                    } else {
                        let placeholder = widget::container(widget::text::body(self.language.library_extracting_frame()))
                            .width(Length::Fixed(240.0))
                            .height(Length::Fixed(135.0))
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
                    }.on_press(Message::ApplyWallpaper {
                        video_path: video.path.clone(),
                        output: selected_output.clone(),
                    });

                    let mut action_row = widget::row::with_capacity(3)
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(apply_btn);

                    // Show in file manager
                    action_row = action_row.push(
                        widget::button::icon(widget::icon::from_name("folder-symbolic"))
                            .on_press(Message::ShowInFileManager(video.path.clone()))
                    );

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

            cards_column.into()
        });

        let main_layout = widget::column::with_capacity(2)
            .spacing(14)
            .push(top_section)
            .push(widget::scrollable(grid).height(Length::Fill));

        Element::from(main_layout)
    }
}
