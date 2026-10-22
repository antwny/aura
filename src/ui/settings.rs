use crate::app::{AuraApp, Message};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    fn settings_flex_row<'a>(
        &self,
        children: Vec<Element<'a, Message>>,
        spacing: u16,
    ) -> Element<'a, Message> {
        widget::flex_row(children)
            .spacing(spacing)
            .align_items(Alignment::Center)
            .width(Length::Fill)
            .into()
    }

    pub(crate) fn view_settings(&self) -> Element<'_, Message> {
        let mut col = widget::column::with_capacity(8)
            .spacing(if self.window_width < 620.0 { 12 } else { 18 })
            .width(Length::Fill);

        col = col.push(widget::text::title2(self.language.settings_title()));

        col = col.push(self.view_general_section());
        col = col.push(self.view_engine_section());
        col = col.push(self.view_performance_section());
        col = col.push(self.view_switcher_section());
        col = col.push(self.view_rotation_section());
        col = col.push(self.view_appearance_section());
        col = col.push(self.view_folders_section());

        Element::from(widget::scrollable(col).height(Length::Fill))
    }

    /// Section 1: Language, Autostart, and System Tray behavior.
    fn view_general_section(&self) -> Element<'_, Message> {
        let is_gnome = crate::engine::gnome::is_active();
        let section = widget::column::with_capacity(5)
            .spacing(14)
            .padding(if self.window_width < 620.0 { 12 } else { 18 })
            .push(
                widget::row::with_capacity(2)
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(widget::icon::from_name("preferences-system-symbolic"))
                    .push(widget::text::title3(self.language.settings_section_general()))
            )
            .push(
                widget::column::with_capacity(3)
                    .spacing(8)
                    .push(widget::text::body(self.language.settings_lang_title()))
                    .push(widget::text::caption(self.language.settings_lang_desc()))
                    .push(
                        self.settings_flex_row(vec![
                                if self.language == crate::i18n::Language::Es {
                                    widget::button::suggested("Español")
                                } else {
                                    widget::button::standard("Español")
                                }
                                .on_press(Message::SetLanguage(crate::i18n::Language::Es))
                                .into(),
                                if self.language == crate::i18n::Language::En {
                                    widget::button::suggested("English")
                                } else {
                                    widget::button::standard("English")
                                }
                                .on_press(Message::SetLanguage(crate::i18n::Language::En))
                                .into(),
                                if self.language == crate::i18n::Language::Fr {
                                    widget::button::suggested("Français")
                                } else {
                                    widget::button::standard("Français")
                                }
                                .on_press(Message::SetLanguage(crate::i18n::Language::Fr))
                                .into(),
                            ], 10)
                    )
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(16)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.autostart_active).on_toggle(Message::ToggleAutostart))
                    .push(widget::text::body(self.language.settings_autostart_desktop(is_gnome)))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(16)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.keep_running_on_close).on_toggle(Message::ToggleKeepRunningOnClose))
                    .push(widget::text::body(self.language.settings_keep_running()))
            );

        widget::container(section)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into()
    }

    /// Section 2: Desktop environment detection, active backend, Hanabi diagnostics on GNOME, and GPU preferences.
    fn view_engine_section(&self) -> Element<'_, Message> {
        let is_gnome = crate::engine::gnome::is_active();
        let mut section = widget::column::with_capacity(9)
            .spacing(14)
            .padding(if self.window_width < 620.0 { 12 } else { 18 })
            .push(
                widget::row::with_capacity(2)
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(widget::icon::from_name("applications-graphics-symbolic"))
                    .push(widget::text::title3(self.language.settings_section_engine()))
            );

        let env_name = if is_gnome { "GNOME Wayland (Mutter)" } else { "COSMIC Desktop (Wayland)" };
        let engine_name = if is_gnome { "GNOME Native (GSettings + Hanabi)" } else { "mpvpaper (Wayland Layer Shell)" };

        let desktop_status: Element<'_, Message> = if self.window_width < 620.0 {
            widget::column::with_capacity(2)
                .spacing(4)
                .push(widget::text::caption(self.language.settings_engine_desktop_lbl()))
                .push(
                    widget::container(widget::text::body(env_name).size(13))
                        .padding([4, 10])
                        .class(cosmic::theme::Container::Card),
                )
                .into()
        } else {
            widget::row::with_capacity(2)
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .push(widget::text::caption(self.language.settings_engine_desktop_lbl()))
                    .push(
                        widget::container(widget::text::body(env_name).size(13))
                            .padding([4, 10])
                            .class(cosmic::theme::Container::Card)
                    )
                    .into()
        };
        let backend_status: Element<'_, Message> = if self.window_width < 620.0 {
            widget::column::with_capacity(2)
                .spacing(4)
                .push(widget::text::caption(self.language.settings_engine_backend_lbl()))
                .push(
                    widget::container(widget::text::body(engine_name).size(13))
                        .padding([4, 10])
                        .class(cosmic::theme::Container::Card),
                )
                .into()
        } else {
            widget::row::with_capacity(2)
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .push(widget::text::caption(self.language.settings_engine_backend_lbl()))
                    .push(
                        widget::container(widget::text::body(engine_name).size(13))
                            .padding([4, 10])
                            .class(cosmic::theme::Container::Card)
                    )
                    .into()
        };
        let status_box = self.settings_flex_row(vec![desktop_status, backend_status], 16);

        section = section.push(status_box);

        if is_gnome {
            let hanabi_ok = crate::engine::gnome::hanabi_installed();
            let diagnostic_row: Element<'_, Message> = if hanabi_ok {
                widget::row::with_capacity(2)
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(widget::icon::from_name("emblem-ok-symbolic"))
                    .push(widget::text::body(self.language.settings_engine_hanabi_status_ok()))
                    .into()
            } else {
                self.settings_flex_row(vec![
                    widget::icon::from_name("dialog-warning-symbolic").into(),
                    widget::text::body(self.language.settings_engine_hanabi_status_missing()).width(Length::Fill).into(),
                    widget::button::suggested(self.language.settings_engine_hanabi_install_btn())
                        .leading_icon(widget::icon::from_name("software-update-available-symbolic"))
                        .on_press(Message::OpenHanabiPage)
                        .into(),
                ], 12)
            };

            section = section.push(
                widget::container(diagnostic_row)
                    .padding(if self.window_width < 620.0 { 8 } else { 12 })
                    .width(Length::Fill)
                    .class(cosmic::theme::Container::Card)
            );
        }

        section = section
            .push(widget::text::body(self.language.settings_hwdec_title()))
            .push(widget::text::caption(self.language.settings_hwdec_desc()))
            .push(
                self.settings_flex_row(vec![
                        if self.config.hwdec == "auto-safe" {
                            widget::button::suggested("Auto (auto-safe)")
                        } else {
                            widget::button::standard("Auto (auto-safe)")
                        }
                        .on_press(Message::SelectHwdec("auto-safe".into()))
                        .into(),
                        if self.config.hwdec == "vaapi" {
                            widget::button::suggested("Intel/AMD (VA-API)")
                        } else {
                            widget::button::standard("Intel/AMD (VA-API)")
                        }
                        .on_press(Message::SelectHwdec("vaapi".into()))
                        .into(),
                        if self.config.hwdec == "nvdec" {
                            widget::button::suggested("NVIDIA (NVDEC)")
                        } else {
                            widget::button::standard("NVIDIA (NVDEC)")
                        }
                        .on_press(Message::SelectHwdec("nvdec".into()))
                        .into(),
                        if self.config.hwdec == "no" {
                            widget::button::suggested("CPU (Software)")
                        } else {
                            widget::button::standard("CPU (Software)")
                        }
                        .on_press(Message::SelectHwdec("no".into()))
                        .into(),
                    ], 8)
            )
            .push(widget::text::body(self.language.settings_gpu_card_title()))
            .push(widget::text::caption(self.language.settings_gpu_card_desc()))
            .push(
                self.settings_flex_row(vec![
                        if self.config.gpu_preference == "auto" {
                            widget::button::suggested(self.language.settings_gpu_auto())
                        } else {
                            widget::button::standard(self.language.settings_gpu_auto())
                        }
                        .on_press(Message::SelectGpuPreference("auto".into()))
                        .into(),
                        if self.config.gpu_preference == "integrated" {
                            widget::button::suggested(self.language.settings_gpu_integrated())
                        } else {
                            widget::button::standard(self.language.settings_gpu_integrated())
                        }
                        .on_press(Message::SelectGpuPreference("integrated".into()))
                        .into(),
                        if self.config.gpu_preference == "discrete" {
                            widget::button::suggested(self.language.settings_gpu_discrete())
                        } else {
                            widget::button::standard(self.language.settings_gpu_discrete())
                        }
                        .on_press(Message::SelectGpuPreference("discrete".into()))
                        .into(),
                    ], 8)
            );

        widget::container(section)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into()
    }

    /// Section 3: Smart pause, desktop hidden auto pause, and battery saver.
    fn view_performance_section(&self) -> Element<'_, Message> {
        let section = widget::column::with_capacity(5)
            .spacing(14)
            .padding(if self.window_width < 620.0 { 12 } else { 18 })
            .push(
                widget::row::with_capacity(2)
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(widget::icon::from_name("process-working-symbolic"))
                    .push(widget::text::title3(self.language.settings_section_performance()))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(16)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.smart_pause).on_toggle(Message::ToggleSmartPause))
                    .push(widget::text::body(self.language.settings_smart_pause()))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(16)
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
                    .spacing(16)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.pause_on_battery).on_toggle(Message::TogglePauseOnBattery))
                    .push(
                        widget::column::with_capacity(2)
                            .spacing(2)
                            .push(widget::text::body(self.language.settings_battery_title()))
                            .push(widget::text::caption(self.language.settings_battery_desc()))
                    )
            );

        widget::container(section)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into()
    }

    /// Section 4: Quick Switcher HUD styles, docking, filter, and shortcuts.
    fn view_switcher_section(&self) -> Element<'_, Message> {
        let count_favs = self.config.favorites.len();
        let switcher_section = widget::column::with_capacity(7)
            .spacing(14)
            .padding(if self.window_width < 620.0 { 12 } else { 18 })
            .push(
                widget::row::with_capacity(2)
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(widget::icon::from_name("view-carousel-symbolic"))
                    .push(widget::text::title3(self.language.settings_switcher_title()))
            )
            .push(widget::text::caption(self.language.settings_switcher_desc()))
            .push(
                self.settings_flex_row(vec![
                    widget::text::body(self.language.settings_switcher_tray_click_label()).into(),
                        if self.config.tray_click_action != "main_window" {
                            widget::button::suggested(self.language.settings_switcher_tray_opt_switcher())
                        } else {
                            widget::button::standard(self.language.settings_switcher_tray_opt_switcher())
                        }
                        .leading_icon(widget::icon::from_name("view-carousel-symbolic"))
                        .on_press(Message::SelectTrayClickAction("switcher".into()))
                        .into(),
                        if self.config.tray_click_action == "main_window" {
                            widget::button::suggested(self.language.settings_switcher_tray_opt_main())
                        } else {
                            widget::button::standard(self.language.settings_switcher_tray_opt_main())
                        }
                        .leading_icon(widget::icon::from_name("window-symbolic"))
                        .on_press(Message::SelectTrayClickAction("main_window".into()))
                        .into(),
                ], 12)
            )
            .push(
                self.settings_flex_row(vec![
                    widget::text::body(self.language.settings_switcher_filter_label()).into(),
                        if !self.config.switcher_only_favorites {
                            widget::button::suggested(self.language.settings_switcher_filter_all())
                        } else {
                            widget::button::standard(self.language.settings_switcher_filter_all())
                        }
                        .leading_icon(widget::icon::from_name("view-grid-symbolic"))
                        .on_press(Message::ToggleSwitcherOnlyFavorites(false))
                        .into(),
                        if self.config.switcher_only_favorites {
                            widget::button::suggested(format!("{} ({})", self.language.settings_switcher_filter_favs(), count_favs))
                        } else {
                            widget::button::standard(format!("{} ({})", self.language.settings_switcher_filter_favs(), count_favs))
                        }
                        .leading_icon(widget::icon::from_name("emblem-favorite-symbolic"))
                        .on_press(Message::ToggleSwitcherOnlyFavorites(true))
                        .into(),
                ], 12)
            )
            .push(
                self.settings_flex_row(vec![
                    widget::text::body(self.language.settings_switcher_style_label()).into(),
                        if self.config.switcher_style == crate::config::SwitcherStyle::Classic {
                            widget::button::suggested(self.language.settings_switcher_style_classic())
                        } else {
                            widget::button::standard(self.language.settings_switcher_style_classic())
                        }
                        .leading_icon(widget::icon::from_name("view-paged-symbolic"))
                        .on_press(Message::SelectSwitcherStyle(crate::config::SwitcherStyle::Classic))
                        .into(),
                        if self.config.switcher_style == crate::config::SwitcherStyle::Cinematic {
                            widget::button::suggested(self.language.settings_switcher_style_cinematic())
                        } else {
                            widget::button::standard(self.language.settings_switcher_style_cinematic())
                        }
                        .leading_icon(widget::icon::from_name("video-display-symbolic"))
                        .on_press(Message::SelectSwitcherStyle(crate::config::SwitcherStyle::Cinematic))
                        .into(),
                        if self.config.switcher_style == crate::config::SwitcherStyle::Honeycomb {
                            widget::button::suggested(self.language.settings_switcher_style_honeycomb())
                        } else {
                            widget::button::standard(self.language.settings_switcher_style_honeycomb())
                        }
                        .leading_icon(widget::icon::from_name("view-grid-symbolic"))
                        .on_press(Message::SelectSwitcherStyle(crate::config::SwitcherStyle::Honeycomb))
                        .into(),
                ], 8)
            )
            .push(
                if self.config.switcher_style == crate::config::SwitcherStyle::Classic {
                    self.settings_flex_row(vec![
                        widget::text::body(self.language.settings_switcher_position_label()).into(),
                            if self.config.switcher_position == "top" || (self.config.switcher_position != "bottom" && self.config.switcher_position != "left" && self.config.switcher_position != "right") {
                                widget::button::suggested(self.language.settings_switcher_pos_top())
                            } else {
                                widget::button::standard(self.language.settings_switcher_pos_top())
                            }
                            .leading_icon(widget::icon::from_name("go-up-symbolic"))
                            .on_press(Message::SelectSwitcherPosition("top".into()))
                            .into(),
                            if self.config.switcher_position == "bottom" {
                                widget::button::suggested(self.language.settings_switcher_pos_bottom())
                            } else {
                                widget::button::standard(self.language.settings_switcher_pos_bottom())
                            }
                            .leading_icon(widget::icon::from_name("go-down-symbolic"))
                            .on_press(Message::SelectSwitcherPosition("bottom".into()))
                            .into(),
                            if self.config.switcher_position == "left" {
                                widget::button::suggested(self.language.settings_switcher_pos_left())
                            } else {
                                widget::button::standard(self.language.settings_switcher_pos_left())
                            }
                            .leading_icon(widget::icon::from_name("go-previous-symbolic"))
                            .on_press(Message::SelectSwitcherPosition("left".into()))
                            .into(),
                            if self.config.switcher_position == "right" {
                                widget::button::suggested(self.language.settings_switcher_pos_right())
                            } else {
                                widget::button::standard(self.language.settings_switcher_pos_right())
                            }
                            .leading_icon(widget::icon::from_name("go-next-symbolic"))
                            .on_press(Message::SelectSwitcherPosition("right".into()))
                            .into(),
                    ], 8)
                } else if self.config.switcher_style == crate::config::SwitcherStyle::Honeycomb {
                    Element::from(widget::text::caption(self.language.settings_switcher_style_honeycomb_desc()))
                } else {
                    Element::from(widget::text::caption(self.language.settings_switcher_style_cinematic_desc()))
                }
            )
            .push(
                widget::column::with_capacity(3)
                    .spacing(6)
                    .push(widget::text::body(self.language.settings_switcher_shortcut_title()))
                    .push(widget::text::caption(self.language.settings_switcher_shortcut_desc()))
                    .push(
                        self.settings_flex_row(vec![
                                widget::container(widget::text::body("aura switcher").size(13))
                                    .padding([6, 12])
                                    .class(cosmic::theme::Container::Card)
                                    .into(),
                                widget::button::standard(self.language.settings_switcher_copy_cmd())
                                    .leading_icon(widget::icon::from_name("edit-copy-symbolic"))
                                    .on_press(Message::CopySwitcherCommand)
                                    .into(),
                            ], 12)
                    )
            );

        widget::container(switcher_section)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into()
    }

    /// Section 5: Wallpaper Auto-Rotation timing, source, and sequencing.
    fn view_rotation_section(&self) -> Element<'_, Message> {
        let mut rotation_section = widget::column::with_capacity(5)
            .spacing(14)
            .padding(if self.window_width < 620.0 { 12 } else { 18 })
            .push(
                widget::row::with_capacity(2)
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(widget::icon::from_name("media-playlist-repeat-symbolic"))
                    .push(widget::text::title3(self.language.settings_rotation_title()))
            )
            .push(widget::text::caption(self.language.settings_rotation_desc()))
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.rotation).on_toggle(Message::ToggleRotation))
                    .push(widget::text::body(self.language.settings_rotation_toggle()))
            );

        if self.config.rotation {
            let count_favs = self.config.favorites.len();
            let source_row = self.settings_flex_row(vec![
                    widget::text::body(self.language.settings_rotation_source_label()).into(),
                    if !self.config.rotation_only_favorites {
                        widget::button::suggested(self.language.settings_rotation_source_all())
                    } else {
                        widget::button::standard(self.language.settings_rotation_source_all())
                    }
                    .leading_icon(widget::icon::from_name("view-grid-symbolic"))
                    .on_press(Message::ToggleRotationOnlyFavorites(false))
                    .into(),
                    if self.config.rotation_only_favorites {
                        widget::button::suggested(format!("{} ({})", self.language.settings_rotation_source_favs(), count_favs))
                    } else {
                        widget::button::standard(format!("{} ({})", self.language.settings_rotation_source_favs(), count_favs))
                    }
                    .leading_icon(widget::icon::from_name("emblem-favorite-symbolic"))
                    .on_press(Message::ToggleRotationOnlyFavorites(true))
                    .into(),
                ], 8);

            let hint_caption = if self.config.rotation_only_favorites {
                if count_favs == 0 {
                    widget::text::caption(self.language.settings_rotation_favs_empty_hint())
                } else {
                    widget::text::caption(self.language.settings_rotation_favs_active_hint(count_favs))
                }
            } else {
                widget::text::caption(self.language.settings_rotation_desc())
            };

            let mut interval_buttons = vec![
                widget::text::body(self.language.settings_interval_label()).into(),
            ];

            for (mins, label) in [(5, "5 min"), (10, "10 min"), (15, "15 min"), (30, "30 min"), (60, "1 h"), (120, "2 h")] {
                let is_sel = self.config.interval == mins;
                let btn = if is_sel {
                    widget::button::suggested(label)
                } else {
                    widget::button::standard(label)
                }
                .on_press(Message::SelectInterval(mins));
                interval_buttons.push(btn.into());
            }
            let interval_row = self.settings_flex_row(interval_buttons, 8);

            let is_custom = ![5, 10, 15, 30, 60, 120].contains(&self.config.interval);
            let custom_interval_row = self.settings_flex_row(vec![
                    widget::text::body(self.language.settings_interval_custom_label()).into(),
                    widget::text_input(
                        self.language.settings_interval_custom_placeholder(),
                        &self.custom_interval_input,
                    )
                    .on_input(Message::CustomIntervalChanged)
                    .on_submit(|_| Message::ApplyCustomInterval)
                    .width(Length::Fixed(85.0))
                    .into(),
                    widget::text::caption(self.language.settings_interval_mins_unit()).into(),
                    if is_custom {
                        widget::button::suggested(self.language.settings_interval_apply())
                    } else {
                        widget::button::standard(self.language.settings_interval_apply())
                    }
                    .on_press(Message::ApplyCustomInterval)
                    .into(),
                ], 8);

            let order_row = self.settings_flex_row(vec![
                    widget::text::body(self.language.settings_order_label()).into(),
                    if self.config.order == "random" {
                        widget::button::suggested(self.language.settings_order_random())
                    } else {
                        widget::button::standard(self.language.settings_order_random())
                    }
                    .leading_icon(widget::icon::from_name("media-playlist-shuffle-symbolic"))
                    .on_press(Message::SelectRotationOrder("random".into()))
                    .into(),
                    if self.config.order == "sequential" {
                        widget::button::suggested(self.language.settings_order_seq())
                    } else {
                        widget::button::standard(self.language.settings_order_seq())
                    }
                    .leading_icon(widget::icon::from_name("media-playlist-repeat-symbolic"))
                    .on_press(Message::SelectRotationOrder("sequential".into()))
                    .into(),
                ], 8);

            rotation_section = rotation_section
                .push(source_row)
                .push(hint_caption)
                .push(interval_row)
                .push(custom_interval_row)
                .push(order_row);
        }

        widget::container(rotation_section)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into()
    }

    /// Section 6: Dynamic accent theme and system appearance mode.
    fn view_appearance_section(&self) -> Element<'_, Message> {
        let section = widget::column::with_capacity(5)
            .spacing(14)
            .padding(if self.window_width < 620.0 { 12 } else { 18 })
            .push(
                self.settings_flex_row(vec![
                    widget::icon::from_name("preferences-system-symbolic").into(),
                    widget::text::title3(self.language.settings_section_appearance()).into(),
                        widget::container(widget::text::caption(self.language.settings_cosmic_badge()).size(11))
                            .padding([2, 8])
                            .class(cosmic::theme::Container::Card)
                            .into(),
                    ], 10)
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(16)
                    .align_y(Alignment::Center)
                    .push(widget::toggler(self.config.auto_theme).on_toggle(Message::ToggleAutoTheme))
                    .push(
                        widget::column::with_capacity(2)
                            .spacing(2)
                            .push(widget::text::body(self.language.settings_auto_theme()))
                            .push(widget::text::caption(self.language.settings_auto_theme_desc()))
                    )
            )
            .push(
                widget::column::with_capacity(3)
                    .spacing(8)
                    .push(widget::text::body(self.language.settings_theme_mode_title()))
                    .push(
                self.settings_flex_row(vec![
                                if self.config.theme_mode == crate::config::ThemeMode::Auto {
                                    widget::button::suggested(self.language.settings_theme_mode_auto())
                                } else {
                                    widget::button::standard(self.language.settings_theme_mode_auto())
                                }
                                .leading_icon(widget::icon::from_name("view-refresh-symbolic"))
                                .on_press(Message::SelectThemeMode(crate::config::ThemeMode::Auto))
                                .into(),
                                if self.config.theme_mode == crate::config::ThemeMode::Dark {
                                    widget::button::suggested(self.language.settings_theme_mode_dark())
                                } else {
                                    widget::button::standard(self.language.settings_theme_mode_dark())
                                }
                                .leading_icon(widget::icon::from_name("weather-clear-night-symbolic"))
                                .on_press(Message::SelectThemeMode(crate::config::ThemeMode::Dark))
                                .into(),
                                if self.config.theme_mode == crate::config::ThemeMode::Light {
                                    widget::button::suggested(self.language.settings_theme_mode_light())
                                } else {
                                    widget::button::standard(self.language.settings_theme_mode_light())
                                }
                                .leading_icon(widget::icon::from_name("weather-clear-symbolic"))
                                .on_press(Message::SelectThemeMode(crate::config::ThemeMode::Light))
                                .into(),
                                if self.config.theme_mode == crate::config::ThemeMode::Manual {
                                    widget::button::suggested(self.language.settings_theme_mode_manual())
                                } else {
                                    widget::button::standard(self.language.settings_theme_mode_manual())
                                }
                                .leading_icon(widget::icon::from_name("preferences-system-symbolic"))
                                .on_press(Message::SelectThemeMode(crate::config::ThemeMode::Manual))
                                .into(),
                            ], 8)
                    )
                    .push(widget::text::caption(match self.config.theme_mode {
                        crate::config::ThemeMode::Auto => self.language.settings_theme_mode_auto_desc(),
                        crate::config::ThemeMode::Dark => self.language.settings_theme_mode_dark_desc(),
                        crate::config::ThemeMode::Light => self.language.settings_theme_mode_light_desc(),
                        crate::config::ThemeMode::Manual => self.language.settings_theme_mode_manual_desc(),
                    }))
            );

        widget::container(section)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into()
    }

    /// Section 7: Management of monitored filesystem directories.
    fn view_folders_section(&self) -> Element<'_, Message> {
        let mut folders_box = widget::column::with_capacity(self.config.dirs.len() + 2)
            .spacing(12)
            .padding(if self.window_width < 620.0 { 12 } else { 18 });

        let folder_heading = widget::row::with_capacity(2)
            .spacing(10)
            .align_y(Alignment::Center)
            .push(widget::icon::from_name("folder-symbolic"))
            .push(widget::text::title3(self.language.settings_monitored_folders()));

        if self.window_width < 620.0 {
            folders_box = folders_box
                .push(folder_heading)
                .push(
                    widget::button::suggested(self.language.settings_btn_add_folder())
                        .leading_icon(widget::icon::from_name("list-add-symbolic"))
                        .on_press(Message::PickFolder),
                );
        } else {
            folders_box = folders_box.push(
                widget::row::with_capacity(2)
                .spacing(16)
                .align_y(Alignment::Center)
                .push(folder_heading.width(Length::Fill))
                .push(
                    widget::button::suggested(self.language.settings_btn_add_folder())
                        .leading_icon(widget::icon::from_name("list-add-symbolic"))
                        .on_press(Message::PickFolder)
                )
            );
        }

        for dir in &self.config.dirs {
            let dir_clone = dir.clone();
            let dir_row = self.settings_flex_row(vec![
                    widget::text::body(dir).width(Length::Fill).into(),
                    widget::button::destructive(self.language.settings_btn_delete())
                        .leading_icon(widget::icon::from_name("user-trash-symbolic"))
                        .on_press(Message::RemoveFolder(dir_clone))
                        .into(),
                ], 12);
            folders_box = folders_box.push(dir_row);
        }

        widget::container(folders_box)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into()
    }
}
