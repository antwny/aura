use crate::app::{AuraApp, Message};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    pub(crate) fn view_settings(&self) -> Element<'_, Message> {
        let mut col = widget::column::with_capacity(7)
            .spacing(18)
            .width(Length::Fill);

        col = col.push(widget::text::title2(self.language.settings_title()));

        // 1. Language selection section
        let lang_section = widget::column::with_capacity(3)
            .spacing(12)
            .padding(16)
            .push(widget::text::title3(self.language.settings_lang_title()))
            .push(widget::text::caption(self.language.settings_lang_desc()))
            .push(
                widget::row::with_capacity(2)
                    .spacing(12)
                    .push(
                        if self.language == crate::i18n::Language::Es {
                            widget::button::suggested("Español")
                        } else {
                            widget::button::standard("Español")
                        }
                        .on_press(Message::SetLanguage(crate::i18n::Language::Es))
                    )
                    .push(
                        if self.language == crate::i18n::Language::En {
                            widget::button::suggested("English")
                        } else {
                            widget::button::standard("English")
                        }
                        .on_press(Message::SetLanguage(crate::i18n::Language::En))
                    )
            );

        col = col.push(widget::container(lang_section).width(Length::Fill));

        // 2. General integrations
        let general_section = widget::column::with_capacity(7)
            .spacing(14)
            .padding(16)
            .push(widget::text::title3(self.language.settings_integration_title()))
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.autostart_active).on_toggle(Message::ToggleAutostart))
                    .push(widget::text::body(self.language.settings_autostart()))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.smart_pause).on_toggle(Message::ToggleSmartPause))
                    .push(widget::text::body(self.language.settings_smart_pause()))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.auto_pause).on_toggle(Message::ToggleAutoPause))
                    .push(
                        widget::column::with_capacity(2)
                            .spacing(2)
                            .push(widget::text::body(self.language.settings_auto_pause()))
                            .push(widget::text::caption(self.language.settings_auto_pause_desc()))
                    )
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.pause_on_battery).on_toggle(Message::TogglePauseOnBattery))
                    .push(
                        widget::column::with_capacity(2)
                            .spacing(2)
                            .push(widget::text::body(self.language.settings_battery_title()))
                            .push(widget::text::caption(self.language.settings_battery_desc()))
                    )
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.keep_running_on_close).on_toggle(Message::ToggleKeepRunningOnClose))
                    .push(widget::text::body(self.language.settings_keep_running()))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.auto_theme).on_toggle(Message::ToggleAutoTheme))
                    .push(widget::text::body(self.language.settings_auto_theme()))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.auto_dark).on_toggle(Message::ToggleAutoDark))
                    .push(widget::text::body(self.language.settings_auto_dark()))
            );

        col = col.push(widget::container(general_section).width(Length::Fill));

        // 3. Playlist Auto-Rotation section
        let mut rotation_section = widget::column::with_capacity(5)
            .spacing(14)
            .padding(16)
            .push(widget::text::title3(self.language.settings_rotation_title()))
            .push(widget::text::caption(self.language.settings_rotation_desc()))
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.rotation).on_toggle(Message::ToggleRotation))
                    .push(widget::text::body(self.language.settings_rotation_toggle()))
            );

        if self.config.rotation {
            // Interval options: 5, 10, 15, 30, 60, 120 minutes
            let mut interval_row = widget::row::with_capacity(7)
                .spacing(8)
                .align_y(Alignment::Center)
                .push(widget::text::body(self.language.settings_interval_label()));

            for (mins, label) in [(5, "5 min"), (10, "10 min"), (15, "15 min"), (30, "30 min"), (60, "1 h"), (120, "2 h")] {
                let is_sel = self.config.interval == mins;
                let btn = if is_sel {
                    widget::button::suggested(label)
                } else {
                    widget::button::standard(label)
                }
                .on_press(Message::SelectInterval(mins));
                interval_row = interval_row.push(btn);
            }

            // Order options: Random vs Sequential
            let order_row = widget::row::with_capacity(3)
                .spacing(8)
                .align_y(Alignment::Center)
                .push(widget::text::body(self.language.settings_order_label()))
                .push(
                    if self.config.order == "random" {
                        widget::button::suggested(self.language.settings_order_random())
                    } else {
                        widget::button::standard(self.language.settings_order_random())
                    }
                    .leading_icon(widget::icon::from_name("media-playlist-shuffle-symbolic"))
                    .on_press(Message::SelectRotationOrder("random".into()))
                )
                .push(
                    if self.config.order == "sequential" {
                        widget::button::suggested(self.language.settings_order_seq())
                    } else {
                        widget::button::standard(self.language.settings_order_seq())
                    }
                    .leading_icon(widget::icon::from_name("media-playlist-repeat-symbolic"))
                    .on_press(Message::SelectRotationOrder("sequential".into()))
                );

            // Source options: All wallpapers vs Only favorites
            let count_favs = self.config.favorites.len();
            let source_row = widget::row::with_capacity(3)
                .spacing(8)
                .align_y(Alignment::Center)
                .push(widget::text::body(self.language.settings_rotation_source_label()))
                .push(
                    if !self.config.rotation_only_favorites {
                        widget::button::suggested(self.language.settings_rotation_source_all())
                    } else {
                        widget::button::standard(self.language.settings_rotation_source_all())
                    }
                    .leading_icon(widget::icon::from_name("view-grid-symbolic"))
                    .on_press(Message::ToggleRotationOnlyFavorites(false))
                )
                .push(
                    if self.config.rotation_only_favorites {
                        widget::button::suggested(format!("{} ({})", self.language.settings_rotation_source_favs(), count_favs))
                    } else {
                        widget::button::standard(format!("{} ({})", self.language.settings_rotation_source_favs(), count_favs))
                    }
                    .leading_icon(widget::icon::from_name("emblem-favorite-symbolic"))
                    .on_press(Message::ToggleRotationOnlyFavorites(true))
                );

            let hint_caption = if self.config.rotation_only_favorites {
                if count_favs == 0 {
                    widget::text::caption(self.language.settings_rotation_favs_empty_hint())
                } else {
                    widget::text::caption(self.language.settings_rotation_favs_active_hint(count_favs))
                }
            } else {
                widget::text::caption(self.language.settings_rotation_desc())
            };

            rotation_section = rotation_section.push(source_row).push(hint_caption).push(interval_row).push(order_row);
        }

        col = col.push(widget::container(rotation_section).width(Length::Fill));

        // 4. Quick Switcher HUD section
        let count_favs = self.config.favorites.len();
        let switcher_section = widget::column::with_capacity(6)
            .spacing(14)
            .padding(16)
            .push(widget::text::title3(self.language.settings_switcher_title()))
            .push(widget::text::caption(self.language.settings_switcher_desc()))
            .push(
                widget::row::with_capacity(3)
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .push(widget::text::body(self.language.settings_switcher_tray_click_label()))
                    .push(
                        if self.config.tray_click_action != "main_window" {
                            widget::button::suggested(self.language.settings_switcher_tray_opt_switcher())
                        } else {
                            widget::button::standard(self.language.settings_switcher_tray_opt_switcher())
                        }
                        .leading_icon(widget::icon::from_name("view-carousel-symbolic"))
                        .on_press(Message::SelectTrayClickAction("switcher".into()))
                    )
                    .push(
                        if self.config.tray_click_action == "main_window" {
                            widget::button::suggested(self.language.settings_switcher_tray_opt_main())
                        } else {
                            widget::button::standard(self.language.settings_switcher_tray_opt_main())
                        }
                        .leading_icon(widget::icon::from_name("window-symbolic"))
                        .on_press(Message::SelectTrayClickAction("main_window".into()))
                    )
            )
            .push(
                widget::row::with_capacity(3)
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .push(widget::text::body(self.language.settings_switcher_filter_label()))
                    .push(
                        if !self.config.switcher_only_favorites {
                            widget::button::suggested(self.language.settings_switcher_filter_all())
                        } else {
                            widget::button::standard(self.language.settings_switcher_filter_all())
                        }
                        .leading_icon(widget::icon::from_name("view-grid-symbolic"))
                        .on_press(Message::ToggleSwitcherOnlyFavorites(false))
                    )
                    .push(
                        if self.config.switcher_only_favorites {
                            widget::button::suggested(format!("{} ({})", self.language.settings_switcher_filter_favs(), count_favs))
                        } else {
                            widget::button::standard(format!("{} ({})", self.language.settings_switcher_filter_favs(), count_favs))
                        }
                        .leading_icon(widget::icon::from_name("emblem-favorite-symbolic"))
                        .on_press(Message::ToggleSwitcherOnlyFavorites(true))
                    )
            )
            .push(
                widget::row::with_capacity(5)
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(widget::text::body(self.language.settings_switcher_position_label()))
                    .push(
                        if self.config.switcher_position == "top" || (self.config.switcher_position != "bottom" && self.config.switcher_position != "left" && self.config.switcher_position != "right") {
                            widget::button::suggested(self.language.settings_switcher_pos_top())
                        } else {
                            widget::button::standard(self.language.settings_switcher_pos_top())
                        }
                        .leading_icon(widget::icon::from_name("go-up-symbolic"))
                        .on_press(Message::SelectSwitcherPosition("top".into()))
                    )
                    .push(
                        if self.config.switcher_position == "bottom" {
                            widget::button::suggested(self.language.settings_switcher_pos_bottom())
                        } else {
                            widget::button::standard(self.language.settings_switcher_pos_bottom())
                        }
                        .leading_icon(widget::icon::from_name("go-down-symbolic"))
                        .on_press(Message::SelectSwitcherPosition("bottom".into()))
                    )
                    .push(
                        if self.config.switcher_position == "left" {
                            widget::button::suggested(self.language.settings_switcher_pos_left())
                        } else {
                            widget::button::standard(self.language.settings_switcher_pos_left())
                        }
                        .leading_icon(widget::icon::from_name("go-previous-symbolic"))
                        .on_press(Message::SelectSwitcherPosition("left".into()))
                    )
                    .push(
                        if self.config.switcher_position == "right" {
                            widget::button::suggested(self.language.settings_switcher_pos_right())
                        } else {
                            widget::button::standard(self.language.settings_switcher_pos_right())
                        }
                        .leading_icon(widget::icon::from_name("go-next-symbolic"))
                        .on_press(Message::SelectSwitcherPosition("right".into()))
                    )
            )
            .push(
                widget::column::with_capacity(3)
                    .spacing(6)
                    .push(widget::text::body(self.language.settings_switcher_shortcut_title()))
                    .push(widget::text::caption(self.language.settings_switcher_shortcut_desc()))
                    .push(
                        widget::row::with_capacity(2)
                            .spacing(12)
                            .align_y(Alignment::Center)
                            .push(
                                widget::container(widget::text::body("aura switcher").size(13))
                                    .padding([6, 12])
                                    .class(cosmic::theme::Container::Card)
                            )
                            .push(
                                widget::button::standard(self.language.settings_switcher_copy_cmd())
                                    .leading_icon(widget::icon::from_name("edit-copy-symbolic"))
                                    .on_press(Message::CopySwitcherCommand)
                            )
                    )
            );

        col = col.push(widget::container(switcher_section).width(Length::Fill));

        // 5. Hardware Acceleration (GPU) section
        let hwdec_section = widget::column::with_capacity(3)
            .spacing(12)
            .padding(16)
            .push(widget::text::title3(self.language.settings_hwdec_title()))
            .push(widget::text::caption(self.language.settings_hwdec_desc()))
            .push(
                widget::row::with_capacity(4)
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(
                        if self.config.hwdec == "auto-safe" {
                            widget::button::suggested("Auto (auto-safe)")
                        } else {
                            widget::button::standard("Auto (auto-safe)")
                        }
                        .on_press(Message::SelectHwdec("auto-safe".into()))
                    )
                    .push(
                        if self.config.hwdec == "vaapi" {
                            widget::button::suggested("Intel/AMD (VA-API)")
                        } else {
                            widget::button::standard("Intel/AMD (VA-API)")
                        }
                        .on_press(Message::SelectHwdec("vaapi".into()))
                    )
                    .push(
                        if self.config.hwdec == "nvdec" {
                            widget::button::suggested("NVIDIA (NVDEC)")
                        } else {
                            widget::button::standard("NVIDIA (NVDEC)")
                        }
                        .on_press(Message::SelectHwdec("nvdec".into()))
                    )
                    .push(
                        if self.config.hwdec == "no" {
                            widget::button::suggested("CPU (Software)")
                        } else {
                            widget::button::standard("CPU (Software)")
                        }
                        .on_press(Message::SelectHwdec("no".into()))
                    )
            );

        col = col.push(widget::container(hwdec_section).width(Length::Fill));

        // 5. Monitored folders section
        let mut folders_box = widget::column::with_capacity(self.config.dirs.len() + 2)
            .spacing(12)
            .padding(16);

        folders_box = folders_box.push(
            widget::row::with_capacity(2)
                .spacing(16)
                .align_y(Alignment::Center)
                .push(widget::text::title3(self.language.settings_monitored_folders()).width(Length::Fill))
                .push(
                    widget::button::suggested(self.language.settings_btn_add_folder())
                        .leading_icon(widget::icon::from_name("folder-symbolic"))
                        .on_press(Message::PickFolder)
                )
        );

        for dir in &self.config.dirs {
            let dir_clone = dir.clone();
            let dir_row = widget::row::with_capacity(2)
                .spacing(12)
                .align_y(Alignment::Center)
                .push(widget::text::body(dir).width(Length::Fill))
                .push(
                    widget::button::destructive(self.language.settings_btn_delete())
                        .on_press(Message::RemoveFolder(dir_clone))
                );
            folders_box = folders_box.push(dir_row);
        }

        col = col.push(widget::container(folders_box).width(Length::Fill));

        // 6. Persistence info card
        let info_card = widget::column::with_capacity(2)
            .spacing(6)
            .padding(14)
            .push(widget::text::title3(self.language.settings_persistence_title()))
            .push(widget::text::caption(self.language.settings_persistence_desc()));

        col = col.push(widget::container(info_card).width(Length::Fill));

        Element::from(widget::scrollable(col).height(Length::Fill))
    }
}
