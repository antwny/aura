use crate::app::{AuraApp, Message, SettingsCategory};
use crate::engine::DesktopEnvironment;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

fn inactive_category_button_class() -> cosmic::theme::Button {
    cosmic::theme::Button::Custom {
        active: Box::new(|_, theme| {
            let mut style = widget::button::Style::new();
            let c = theme.cosmic().on_bg_color();
            let color = cosmic::iced::Color::from_rgb(c.red, c.green, c.blue);
            style.text_color = Some(color);
            style.icon_color = Some(color);
            style
        }),
        disabled: Box::new(|theme| {
            let mut style = widget::button::Style::new();
            let c = theme.cosmic().on_bg_color();
            let color = cosmic::iced::Color::from_rgba(c.red, c.green, c.blue, 0.4);
            style.text_color = Some(color);
            style.icon_color = Some(color);
            style
        }),
        hovered: Box::new(|_, theme| {
            let mut style = widget::button::Style::new();
            let c = theme.cosmic().on_bg_color();
            let color = cosmic::iced::Color::from_rgb(c.red, c.green, c.blue);
            style.text_color = Some(color);
            style.icon_color = Some(color);
            let h = theme.cosmic().icon_button.hover;
            style.background = Some(cosmic::iced::Background::Color(
                cosmic::iced::Color::from_rgba(h.red, h.green, h.blue, h.alpha),
            ));
            style.border_radius = cosmic::iced::border::Radius::from(
                theme.cosmic().corner_radii.radius_xl,
            );
            style
        }),
        pressed: Box::new(|_, theme| {
            let mut style = widget::button::Style::new();
            let c = theme.cosmic().on_bg_color();
            let color = cosmic::iced::Color::from_rgb(c.red, c.green, c.blue);
            style.text_color = Some(color);
            style.icon_color = Some(color);
            let p = theme.cosmic().icon_button.pressed;
            style.background = Some(cosmic::iced::Background::Color(
                cosmic::iced::Color::from_rgba(p.red, p.green, p.blue, p.alpha),
            ));
            style.border_radius = cosmic::iced::border::Radius::from(
                theme.cosmic().corner_radii.radius_xl,
            );
            style
        }),
    }
}

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
        let spacing = if self.window_width < 620.0 { 12 } else { 18 };
        let col = widget::column::with_capacity(3)
            .spacing(spacing)
            .width(Length::Fill)
            .height(Length::Fill)
            .push(widget::text::title2(self.language.settings_title()))
            .push(self.view_settings_category_chips())
            .push(self.view_selected_settings_category());

        col.into()
    }

    fn settings_category_label(&self, category: SettingsCategory) -> &'static str {
        match category {
            SettingsCategory::General => self.language.settings_section_general(),
            SettingsCategory::Engine => self.language.settings_section_engine(),
            SettingsCategory::Hud => self.language.settings_switcher_title(),
            SettingsCategory::Rotation => self.language.settings_rotation_title(),
            SettingsCategory::Appearance => self.language.settings_section_appearance(),
        }
    }

    fn settings_category_icon(category: SettingsCategory) -> &'static str {
        match category {
            SettingsCategory::General => "preferences-system-symbolic",
            SettingsCategory::Engine => "applications-graphics-symbolic",
            SettingsCategory::Hud => "view-grid-symbolic",
            SettingsCategory::Rotation => "media-playlist-repeat-symbolic",
            SettingsCategory::Appearance => "weather-clear-symbolic",
        }
    }

    fn view_settings_category_chips(&self) -> Element<'_, Message> {
        let mut categories = Vec::with_capacity(SettingsCategory::ALL.len());
        for category in SettingsCategory::ALL {
            let label = self.settings_category_label(category);
            let icon = widget::icon::from_name(Self::settings_category_icon(category));
            let button = if category == self.selected_settings_category {
                widget::button::suggested(label)
            } else {
                widget::button::text(label).class(inactive_category_button_class())
            }
            .leading_icon(icon)
            .on_press(Message::SelectSettingsCategory(category));
            categories.push(button.into());
        }

        widget::container(
            widget::flex_row(categories)
                .spacing(8)
                .align_items(Alignment::Center),
        )
            .class(cosmic::theme::Container::Card)
            .padding(8)
            .width(Length::Fill)
            .into()
    }

    fn view_selected_settings_category(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = match self.selected_settings_category {
            SettingsCategory::General => widget::column::with_capacity(2)
                .spacing(18)
                .push(self.view_general_section())
                .push(self.view_folders_section())
                .into(),
            SettingsCategory::Engine => widget::column::with_capacity(2)
                .spacing(18)
                .push(self.view_engine_section())
                .push(self.view_performance_section())
                .into(),
            SettingsCategory::Hud => self.view_switcher_section().into(),
            SettingsCategory::Rotation => self.view_rotation_section().into(),
            SettingsCategory::Appearance => self.view_appearance_section().into(),
        };

        widget::scrollable(content)
            .id(self.settings_scroll_id.clone())
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Section 1: Language, Autostart, and System Tray behavior.
    fn view_general_section(&self) -> Element<'_, Message> {
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
                    .push(widget::text::body(self.language.settings_autostart_desktop()))
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
        let backend = self.backend();
        let is_gnome = backend == crate::engine::Backend::Gnome;
        let mut section = widget::column::with_capacity(12)
            .spacing(14)
            .padding(if self.window_width < 620.0 { 12 } else { 18 })
            .push(
                widget::row::with_capacity(2)
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(widget::icon::from_name("applications-graphics-symbolic"))
                    .push(widget::text::title3(self.language.settings_section_engine()))
            );

        let env_name = self.desktop_environment().pretty_name();
        let engine_name = backend.label().to_string();

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
        if let Some(diagnostic) = crate::engine::backend::backend_diagnostic(self.config.backend) {
            section = section.push(widget::text::caption(diagnostic));
        }

        let mut backend_buttons = vec![
            widget::text::body(self.language.settings_backend_choice_lbl()).into(),
        ];
        for preference in [
            crate::config::BackendPreference::Auto,
            crate::config::BackendPreference::Mpvpaper,
            crate::config::BackendPreference::Gnome,
            crate::config::BackendPreference::Kde,
            crate::config::BackendPreference::Static,
        ] {
            let label = match preference {
                crate::config::BackendPreference::Auto => self.language.settings_backend_auto(),
                crate::config::BackendPreference::Mpvpaper => "mpvpaper",
                crate::config::BackendPreference::Gnome => "GNOME",
                crate::config::BackendPreference::Kde => "KDE",
                crate::config::BackendPreference::Static => self.language.settings_backend_static(),
            };
            let button = if self.config.backend == preference {
                widget::button::suggested(label)
            } else {
                widget::button::standard(label)
            };
            backend_buttons.push(
                button
                    .on_press(Message::SelectBackendPreference(preference))
                    .into(),
            );
        }
        section = section.push(self.settings_flex_row(backend_buttons, 8));

        if backend == crate::engine::Backend::Kde {
            let dbus_ok = crate::engine::kde::dbus_available();
            let plugin_ok = crate::engine::kde::plugin_installed();
            let diagnostic_row: Element<'_, Message> = if !dbus_ok {
                widget::row::with_capacity(2)
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(widget::icon::from_name("dialog-warning-symbolic"))
                    .push(
                        widget::text::body(self.language.settings_kde_dbus_missing())
                            .width(Length::Fill),
                    )
                    .into()
            } else if plugin_ok {
                widget::row::with_capacity(2)
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(widget::icon::from_name("emblem-ok-symbolic"))
                    .push(widget::text::body(self.language.settings_kde_plugin_ok()))
                    .into()
            } else {
                self.settings_flex_row(vec![
                    widget::icon::from_name("dialog-warning-symbolic").into(),
                    widget::text::body(self.language.settings_kde_plugin_missing())
                        .width(Length::Fill)
                        .into(),
                    widget::button::suggested(self.language.settings_kde_plugin_install_btn())
                        .leading_icon(widget::icon::from_name("software-update-available-symbolic"))
                        .on_press(Message::OpenKdePluginPage)
                        .into(),
                ], 12)
            };
            section = section.push(
                widget::container(diagnostic_row)
                    .padding(if self.window_width < 620.0 { 8 } else { 12 })
                    .width(Length::Fill)
                    .class(cosmic::theme::Container::Card),
            );
        }

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

    /// Section 3: auto-pause, smart pause and battery saving. The controls
    /// adapt to the active engine because COSMIC, KDE and GNOME handle window
    /// occlusion very differently.
    fn view_performance_section(&self) -> Element<'_, Message> {
        let backend = self.backend();
        let desktop = self.desktop_environment();
        let kde_ready = backend != crate::engine::Backend::Kde
            || (crate::engine::kde::plugin_installed()
                && crate::engine::kde::dbus_available());
        let engine_supports_pause = match backend {
            crate::engine::Backend::Mpvpaper => true,
            crate::engine::Backend::Kde => kde_ready,
            crate::engine::Backend::Gnome | crate::engine::Backend::Static => false,
        };

        let (platform_icon, platform_text, platform_warning) = match backend {
            crate::engine::Backend::Gnome => (
                "dialog-warning-symbolic",
                self.language.settings_auto_pause_platform_gnome(),
                true,
            ),
            crate::engine::Backend::Static => (
                "dialog-information-symbolic",
                self.language.settings_auto_pause_platform_static(),
                false,
            ),
            crate::engine::Backend::Kde => {
                if kde_ready {
                    (
                        "emblem-ok-symbolic",
                        self.language.settings_auto_pause_platform_kde(),
                        false,
                    )
                } else if !crate::engine::kde::dbus_available() {
                    (
                        "dialog-warning-symbolic",
                        self.language.settings_auto_pause_platform_kde_no_dbus(),
                        true,
                    )
                } else {
                    (
                        "dialog-warning-symbolic",
                        self.language.settings_auto_pause_platform_kde_missing(),
                        true,
                    )
                }
            }
            crate::engine::Backend::Mpvpaper => match desktop {
                crate::engine::DesktopEnvironment::Cosmic => (
                    "emblem-ok-symbolic",
                    self.language.settings_auto_pause_platform_cosmic(),
                    false,
                ),
                crate::engine::DesktopEnvironment::Hyprland
                | crate::engine::DesktopEnvironment::Sway
                | crate::engine::DesktopEnvironment::Niri => (
                    "emblem-ok-symbolic",
                    self.language
                        .settings_auto_pause_platform_mpvpaper_wlroots(),
                    false,
                ),
                _ => (
                    "dialog-information-symbolic",
                    self.language.settings_auto_pause_platform_mpvpaper(),
                    false,
                ),
            },
        };

        let platform_note = widget::container(
            widget::row::with_capacity(2)
                .spacing(10)
                .align_y(Alignment::Start)
                .push(widget::icon::from_name(platform_icon))
                .push(
                    widget::text::caption(platform_text)
                        .width(Length::Fill),
                ),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .class(cosmic::theme::Container::Secondary);

        let auto_pause_header = widget::row::with_capacity(2)
            .spacing(16)
            .align_y(Alignment::Start)
            .push(widget::toggler(self.config.auto_pause).on_toggle(Message::ToggleAutoPause))
            .push(
                widget::column::with_capacity(2)
                    .spacing(2)
                    .push(widget::text::body(self.language.settings_auto_pause()))
                    .push(
                        widget::text::caption(self.language.settings_auto_pause_desc())
                            .width(Length::Fill),
                    ),
            );

        let mut auto_pause_block = widget::column::with_capacity(5)
            .spacing(10)
            .push(auto_pause_header)
            .push(platform_note);

        if self.config.auto_pause
            && engine_supports_pause
            && backend == crate::engine::Backend::Mpvpaper
            && desktop == crate::engine::DesktopEnvironment::Cosmic
        {
            let (status_icon, status_text) = if !self.effective_occluded_outputs().is_empty() {
                (
                    "media-playback-pause-symbolic",
                    self.language.settings_auto_pause_status_paused(),
                )
            } else if !self.is_paused_by_occlusion.is_empty() {
                (
                    "media-playback-start-symbolic",
                    self.language.settings_auto_pause_status_overridden(),
                )
            } else if self.is_paused {
                (
                    "media-playback-pause-symbolic",
                    self.language.settings_auto_pause_status_paused_other(),
                )
            } else {
                (
                    "media-playback-start-symbolic",
                    self.language.settings_auto_pause_status_playing(),
                )
            };
            auto_pause_block = auto_pause_block.push(
                widget::container(
                    widget::row::with_capacity(2)
                        .spacing(10)
                        .align_y(Alignment::Center)
                        .push(widget::icon::from_name(status_icon))
                        .push(
                            widget::text::caption(status_text)
                                .width(Length::Fill),
                        ),
                )
                .padding([6, 10])
                .width(Length::Fill)
                .class(cosmic::theme::Container::Card),
            );
        }

        if !self.config.auto_pause {
            auto_pause_block = auto_pause_block.push(
                widget::text::caption(self.language.settings_auto_pause_disabled_hint())
                    .width(Length::Fill),
            );
        } else if engine_supports_pause {
            let mode_buttons: Vec<Element<'_, Message>> = self
                .language
                .auto_pause_mode_choices()
                .into_iter()
                .map(|(mode, label)| {
                    if self.config.auto_pause_mode == mode {
                        widget::button::suggested(label)
                    } else {
                        widget::button::standard(label)
                    }
                    .on_press(Message::SelectAutoPauseMode(mode))
                    .into()
                })
                .collect();
            auto_pause_block = auto_pause_block
                .push(widget::text::body(self.language.settings_auto_pause_mode()))
                .push(self.settings_flex_row(mode_buttons, 8))
                .push(
                    widget::text::caption(
                        self.language
                            .auto_pause_mode_hint(self.config.auto_pause_mode),
                    )
                    .width(Length::Fill),
                );
        } else if platform_warning {
            // The platform note above already explains why this engine
            // cannot pause by itself; keep the selector out of the way.
            auto_pause_block = auto_pause_block.push(
                widget::text::caption(
                    self.language.settings_auto_pause_unavailable_hint(),
                )
                .width(Length::Fill),
            );
        }

        let section = widget::column::with_capacity(9)
            .spacing(14)
            .padding(if self.window_width < 620.0 { 12 } else { 18 })
            .push(
                widget::row::with_capacity(2)
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(widget::icon::from_name("process-working-symbolic"))
                    .push(widget::text::title3(self.language.settings_section_performance())),
            )
            .push(
                widget::text::caption(self.language.settings_performance_desc())
                    .width(Length::Fill),
            )
            .push(
                widget::container(auto_pause_block)
                    .padding(if self.window_width < 620.0 { 10 } else { 14 })
                    .width(Length::Fill)
                    .class(cosmic::theme::Container::Secondary),
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(16)
                    .align_y(Alignment::Start)
                    .push(
                        widget::toggler(self.config.smart_pause)
                            .on_toggle(Message::ToggleSmartPause),
                    )
                    .push(
                        widget::column::with_capacity(2)
                            .spacing(2)
                            .push(widget::text::body(self.language.settings_smart_pause()))
                            .push(
                                widget::text::caption(
                                    self.language.settings_smart_pause_desc(),
                                )
                                .width(Length::Fill),
                            ),
                    ),
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(16)
                    .align_y(Alignment::Start)
                    .push(
                        widget::toggler(self.config.pause_on_battery)
                            .on_toggle(Message::TogglePauseOnBattery),
                    )
                    .push(
                        widget::column::with_capacity(2)
                            .spacing(2)
                            .push(widget::text::body(self.language.settings_battery_title()))
                            .push(
                                widget::text::caption(self.language.settings_battery_desc())
                                    .width(Length::Fill),
                            ),
                    ),
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
                    .push(widget::icon::from_name("view-grid-symbolic"))
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
                        .leading_icon(widget::icon::from_name("view-grid-symbolic"))
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

            let live_count = self.videos.iter().filter(|video| video.is_video).count();
            let static_count = self.videos.len().saturating_sub(live_count);
            let media_row = self.settings_flex_row(vec![
                widget::text::body(self.language.settings_rotation_media_lbl()).into(),
                if self.config.rotation_media_filter == crate::config::RotationMediaFilter::All {
                    widget::button::suggested(format!("{} ({})", self.language.settings_rotation_media_all(), self.videos.len()))
                } else {
                    widget::button::standard(format!("{} ({})", self.language.settings_rotation_media_all(), self.videos.len()))
                }
                .leading_icon(widget::icon::from_name("view-grid-symbolic"))
                .on_press(Message::SelectRotationMediaFilter(crate::config::RotationMediaFilter::All))
                .into(),
                if self.config.rotation_media_filter == crate::config::RotationMediaFilter::Live {
                    widget::button::suggested(format!("{} ({})", self.language.settings_rotation_media_live(), live_count))
                } else {
                    widget::button::standard(format!("{} ({})", self.language.settings_rotation_media_live(), live_count))
                }
                .leading_icon(widget::icon::from_name("video-x-generic-symbolic"))
                .on_press(Message::SelectRotationMediaFilter(crate::config::RotationMediaFilter::Live))
                .into(),
                if self.config.rotation_media_filter == crate::config::RotationMediaFilter::Static {
                    widget::button::suggested(format!("{} ({})", self.language.settings_rotation_media_static(), static_count))
                } else {
                    widget::button::standard(format!("{} ({})", self.language.settings_rotation_media_static(), static_count))
                }
                .leading_icon(widget::icon::from_name("image-x-generic-symbolic"))
                .on_press(Message::SelectRotationMediaFilter(crate::config::RotationMediaFilter::Static))
                .into(),
            ], 8);

            let pool_count = self.rotation_pool().len();
            let hint_caption = if pool_count == 0 {
                widget::text::caption(self.language.settings_rotation_pool_empty())
            } else if self.config.rotation_only_favorites {
                if count_favs == 0 {
                    widget::text::caption(self.language.settings_rotation_favs_empty_hint())
                } else {
                    widget::text::caption(self.language.settings_rotation_favs_active_hint(count_favs))
                }
            } else {
                widget::text::caption(self.language.settings_rotation_pool_count(pool_count))
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
                .push(media_row)
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
        let auto_theme_desc = if self.desktop_environment() == DesktopEnvironment::Cosmic {
            self.language.settings_auto_theme_desc_cosmic()
        } else {
            self.language.settings_auto_theme_desc_app_only()
        };
        let section = widget::column::with_capacity(5)
            .spacing(14)
            .padding(if self.window_width < 620.0 { 12 } else { 18 })
            .push(
                self.settings_flex_row(vec![
                    widget::icon::from_name("preferences-system-symbolic").into(),
                    widget::text::title3(self.language.settings_section_appearance()).into(),
                        widget::container(widget::text::caption(self.desktop_environment().label()).size(11))
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
                            .push(widget::text::caption(auto_theme_desc))
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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic::widget::button::Catalog;

    #[test]
    fn inactive_category_buttons_use_theme_text_color() {
        let light = cosmic::Theme::light();
        let dark = cosmic::Theme::dark();
        let class = inactive_category_button_class();

        let style_light = <cosmic::Theme as Catalog>::active(&light, false, false, &class);
        let color_light = style_light.text_color.expect("inactive button text color light");
        // In light theme, text must be dark (not white, not accent)
        assert!(color_light.r < 0.5 && color_light.g < 0.5 && color_light.b < 0.5);

        let style_dark = <cosmic::Theme as Catalog>::active(&dark, false, false, &class);
        let color_dark = style_dark.text_color.expect("inactive button text color dark");
        // In dark theme, text must be light/white (not accent)
        assert!(color_dark.r > 0.8 && color_dark.g > 0.8 && color_dark.b > 0.8);
    }
}
