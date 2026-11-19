use crate::app::{AuraApp, EffectsModelStatus, Message};
use crate::config::{SpectrumColorMode, SpectrumPosition, SpectrumStyle};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

fn flex_row<'a>(children: Vec<Element<'a, Message>>, spacing: u16) -> Element<'a, Message> {
    widget::flex_row(children)
        .spacing(spacing)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .into()
}

fn picker_labels(lang: crate::i18n::Language) -> (&'static str, &'static str, &'static str, &'static str, &'static str, &'static str) {
    match lang {
        crate::i18n::Language::Es => (
            "Colores recientes",
            "Copiar",
            "Copiado",
            "Restablecer",
            "Guardar",
            "Cancelar",
        ),
        crate::i18n::Language::Fr => (
            "Couleurs récentes",
            "Copier",
            "Copié",
            "Réinitialiser",
            "Enregistrer",
            "Annuler",
        ),
        crate::i18n::Language::En => (
            "Recent colors",
            "Copy",
            "Copied",
            "Reset",
            "Save",
            "Cancel",
        ),
    }
}

impl AuraApp {
    pub(crate) fn view_effects_page(&self) -> Element<'_, Message> {
        let compact = self.window_width < 620.0;
        let spacing = if compact { 12 } else { 18 };
        let lang = self.language;

        let content = widget::column::with_capacity(5)
            .spacing(spacing)
            .width(Length::Fill)
            .push(
                widget::column::with_capacity(2)
                    .spacing(4)
                    .push(widget::text::title2(lang.settings_section_effects()))
                    .push(widget::text::caption(lang.settings_effects_desc())),
            )
            .push(self.view_effects_renderer_card())
            .push(self.view_spectrum_card())
            .push(self.view_parallax_card());

        widget::scrollable(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn effects_card<'a>(&self, title: &str, icon: &'static str, body: Element<'a, Message>) -> Element<'a, Message> {
        let compact = self.window_width < 620.0;
        widget::container(
            widget::column::with_capacity(3)
                .spacing(14)
                .padding(if compact { 12 } else { 18 })
                .push(
                    widget::row::with_capacity(2)
                        .spacing(10)
                        .align_y(Alignment::Center)
                        .push(widget::icon::from_name(icon))
                        .push(widget::text::title3(title.to_string())),
                )
                .push(body),
        )
        .width(Length::Fill)
        .class(cosmic::theme::Container::Card)
        .into()
    }

    fn view_effects_renderer_card(&self) -> Element<'_, Message> {
        let lang = self.language;
        let supported = crate::effects::renderer_supported();
        let renderer_status = match &self.effects_renderer_status {
            crate::effects::EffectsStatus::Running => {
                lang.settings_effects_renderer_running().to_string()
            }
            crate::effects::EffectsStatus::Disabled => {
                lang.settings_effects_renderer_disabled().to_string()
            }
            crate::effects::EffectsStatus::Failed(error) => {
                format!("{}: {}", lang.settings_effects_renderer_failed(), error)
            }
        };

        let mut body = widget::column::with_capacity(3)
            .spacing(8)
            .push(
                widget::row::with_capacity(4)
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(widget::text::body(format!(
                        "{}: {}",
                        lang.settings_effects_renderer_status(),
                        renderer_status
                    )))
                    .push(
                        widget::button::standard(lang.settings_effects_restart())
                            .on_press(Message::RestartEffects),
                    )
                    .push(
                        widget::button::text(lang.settings_effects_open_log())
                            .on_press(Message::OpenEffectsLog),
                    ),
            )
            .push(widget::text::caption(format!(
                "{}: {}",
                lang.settings_effects_open_log(),
                crate::effects::log_path().display()
            )));

        if !supported {
            body = body.push(
                widget::text::caption(lang.settings_effects_unsupported())
                    .width(Length::Fill),
            );
        }

        self.effects_card(
            lang.settings_effects_renderer_status(),
            "utilities-system-monitor-symbolic",
            body.into(),
        )
    }

    fn view_spectrum_card(&self) -> Element<'_, Message> {
        let lang = self.language;
        let mut body = widget::column::with_capacity(18)
            .spacing(12)
            .push(
                widget::row::with_capacity(2)
                    .spacing(14)
                    .align_y(Alignment::Center)
                    .push(
                        widget::toggler(self.config.spectrum_enabled)
                            .on_toggle(Message::ToggleSpectrum),
                    )
                    .push(
                        widget::column::with_capacity(2)
                            .spacing(3)
                            .push(widget::text::body(lang.settings_effects_spectrum()))
                            .push(
                                widget::text::caption(lang.settings_effects_spectrum_desc())
                                    .width(Length::Fill),
                            ),
                    ),
            );

        if !self.config.spectrum_enabled {
            return self.effects_card(
                lang.settings_effects_spectrum(),
                "audio-x-generic-symbolic",
                body.push(widget::text::caption(lang.settings_effects_spectrum_cava_hint()))
                    .into(),
            );
        }

        let auto = self.config.spectrum_color_mode == SpectrumColorMode::Auto;
        let cava = crate::effects::cava_available();
        body = body.push(
            widget::row::with_capacity(2)
                .spacing(6)
                .align_y(Alignment::Center)
                .push(
                    widget::icon::from_name(if cava {
                        "emblem-ok-symbolic"
                    } else {
                        "dialog-warning-symbolic"
                    })
                    .size(14),
                )
                .push(widget::text::caption(if cava {
                    lang.settings_effects_cava_ready()
                } else {
                    lang.settings_effects_cava_missing()
                })),
        );
        body = body
            .push(widget::divider::horizontal::light())
            .push(widget::text::body(lang.settings_effects_color_mode()))
            .push(flex_row(
                vec![
                    if auto {
                        widget::button::suggested(lang.settings_effects_color_auto())
                    } else {
                        widget::button::standard(lang.settings_effects_color_auto())
                    }
                    .on_press(Message::SetSpectrumColorMode(SpectrumColorMode::Auto))
                    .into(),
                    if auto {
                        widget::button::standard(lang.settings_effects_color_manual())
                    } else {
                        widget::button::suggested(lang.settings_effects_color_manual())
                    }
                    .on_press(Message::SetSpectrumColorMode(SpectrumColorMode::Manual))
                    .into(),
                ],
                8,
            ));

        if auto {
            body = body.push(
                widget::text::caption(lang.settings_effects_color_auto_desc())
                    .width(Length::Fill),
            );
        } else {
            body = body
                .push(
                    widget::text::caption(lang.settings_effects_color_manual_desc())
                        .width(Length::Fill),
                )
                .push(self.view_manual_color_row(1))
                .push(self.view_manual_color_row(2));
        }

        body = body
            .push(widget::divider::horizontal::light())
            .push(widget::text::body(lang.settings_effects_position()))
            .push(flex_row(
                vec![
                    if self.config.spectrum_position == SpectrumPosition::Bottom {
                        widget::button::suggested(lang.settings_effects_position_bottom())
                    } else {
                        widget::button::standard(lang.settings_effects_position_bottom())
                    }
                    .on_press(Message::SetSpectrumPosition(SpectrumPosition::Bottom))
                    .into(),
                    if self.config.spectrum_position == SpectrumPosition::Top {
                        widget::button::suggested(lang.settings_effects_position_top())
                    } else {
                        widget::button::standard(lang.settings_effects_position_top())
                    }
                    .on_press(Message::SetSpectrumPosition(SpectrumPosition::Top))
                    .into(),
                ],
                8,
            ))
            .push(widget::text::body(lang.settings_effects_style()))
            .push(flex_row(
                vec![
                    if self.config.spectrum_style == SpectrumStyle::Bars {
                        widget::button::suggested(lang.settings_effects_style_bars())
                    } else {
                        widget::button::standard(lang.settings_effects_style_bars())
                    }
                    .on_press(Message::SetSpectrumStyle(SpectrumStyle::Bars))
                    .into(),
                    if self.config.spectrum_style == SpectrumStyle::Wave {
                        widget::button::suggested(lang.settings_effects_style_wave())
                    } else {
                        widget::button::standard(lang.settings_effects_style_wave())
                    }
                    .on_press(Message::SetSpectrumStyle(SpectrumStyle::Wave))
                    .into(),
                    if self.config.spectrum_style == SpectrumStyle::Dots {
                        widget::button::suggested(lang.settings_effects_style_dots())
                    } else {
                        widget::button::standard(lang.settings_effects_style_dots())
                    }
                    .on_press(Message::SetSpectrumStyle(SpectrumStyle::Dots))
                    .into(),
                    if self.config.spectrum_style == SpectrumStyle::Mirror {
                        widget::button::suggested(lang.settings_effects_style_mirror())
                    } else {
                        widget::button::standard(lang.settings_effects_style_mirror())
                    }
                    .on_press(Message::SetSpectrumStyle(SpectrumStyle::Mirror))
                    .into(),
                ],
                8,
            ))
            .push(widget::text::caption(format!(
                "{}: {}%",
                lang.settings_effects_height(),
                self.config.spectrum_height_pct
            )))
            .push(flex_row(
                vec![
                    self.height_preset(lang.settings_effects_height_compact(), 8),
                    self.height_preset(lang.settings_effects_height_normal(), 16),
                    self.height_preset(lang.settings_effects_height_large(), 24),
                    self.height_preset(lang.settings_effects_height_huge(), 32),
                ],
                8,
            ))
            .push(widget::slider(
                4u32..=60u32,
                self.config.spectrum_height_pct.clamp(4, 60),
                Message::SetSpectrumHeight,
            ))
            .push(widget::text::caption(lang.settings_effects_height_hint()))
            .push(widget::text::caption(format!(
                "{}: {}",
                if self.config.spectrum_style == SpectrumStyle::Bars {
                    lang.settings_effects_bars()
                } else {
                    lang.settings_effects_bands()
                },
                self.config.spectrum_bars
            )))
            .push(widget::slider(
                8u32..=64u32,
                self.config.spectrum_bars,
                Message::SetSpectrumBars,
            ));

        if matches!(
            self.config.spectrum_style,
            SpectrumStyle::Bars | SpectrumStyle::Mirror
        ) {
            body = body
                .push(widget::text::caption(format!(
                    "{}: {}%",
                    lang.settings_effects_bar_width(),
                    self.config.spectrum_bar_width_pct
                )))
                .push(widget::slider(
                    10u32..=100u32,
                    self.config.spectrum_bar_width_pct,
                    Message::SetSpectrumBarWidth,
                ));
        }

        body = body
            .push(widget::text::caption(format!(
                "{}: {}%",
                lang.settings_effects_opacity(),
                (self.config.spectrum_alpha * 100.0).round() as u32
            )))
            .push(widget::slider(
                10u32..=100u32,
                (self.config.spectrum_alpha * 100.0).round() as u32,
                |value| Message::SetSpectrumAlpha(value as f32 / 100.0),
            ))
            .push(
                widget::row::with_capacity(2)
                    .spacing(14)
                    .align_y(Alignment::Center)
                    .push(
                        widget::toggler(self.config.spectrum_glow)
                            .on_toggle(Message::ToggleSpectrumGlow),
                    )
                    .push(
                        widget::column::with_capacity(2)
                            .spacing(3)
                            .push(widget::text::body(lang.settings_effects_glow()))
                            .push(
                                widget::text::caption(lang.settings_effects_glow_desc())
                                    .width(Length::Fill),
                            ),
                    ),
            )
            .push(widget::text::caption(lang.settings_effects_spectrum_cava_hint()));

        self.effects_card(
            lang.settings_effects_spectrum(),
            "audio-x-generic-symbolic",
            body.into(),
        )
    }

    fn height_preset(&self, label: &str, height: u32) -> Element<'_, Message> {
        let button = if self.config.spectrum_height_pct == height {
            widget::button::suggested(format!("{label} · {height}%"))
        } else {
            widget::button::standard(format!("{label} · {height}%"))
        };
        button.on_press(Message::SetSpectrumHeight(height)).into()
    }

    fn view_manual_color_row(&self, slot: u8) -> Element<'_, Message> {
        let lang = self.language;
        let (label, on_update): (
            &'static str,
            fn(cosmic::widget::color_picker::ColorPickerUpdate) -> Message,
        ) = if slot == 1 {
            (
                lang.settings_effects_color1(),
                Message::SpectrumColor1Update,
            )
        } else {
            (
                lang.settings_effects_color2(),
                Message::SpectrumColor2Update,
            )
        };
        let picker = if slot == 1 {
            &self.spectrum_color1_picker
        } else {
            &self.spectrum_color2_picker
        };

        let (recent_label, copy_label, copied_label, reset_label, save_label, cancel_label) =
            picker_labels(lang);

        let mut column = widget::column::with_capacity(2).spacing(8).push(
            widget::row::with_capacity(3)
                .spacing(12)
                .align_y(Alignment::Center)
                .push(widget::text::body(label).width(Length::Fill))
                .push(
                    picker
                        .picker_button(on_update, None)
                        .width(Length::Fixed(64.0))
                        .height(Length::Fixed(36.0)),
                ),
        );

        if picker.get_is_active() {
            column = column.push(
                picker
                    .builder(on_update)
                    .reset_label(reset_label)
                    .save_label(save_label)
                    .cancel_label(cancel_label)
                    .width(Length::Fill)
                    .build(recent_label, copy_label, copied_label),
            );
        }

        column.into()
    }

    fn view_parallax_card(&self) -> Element<'_, Message> {
        let lang = self.language;
        let mut body = widget::column::with_capacity(8)
            .spacing(8)
            .push(
                widget::row::with_capacity(2)
                    .spacing(14)
                    .align_y(Alignment::Center)
                    .push(
                        widget::toggler(self.config.parallax_enabled)
                            .on_toggle(Message::ToggleParallax),
                    )
                    .push(
                        widget::column::with_capacity(2)
                            .spacing(3)
                            .push(widget::text::body(lang.settings_effects_parallax()))
                            .push(
                                widget::text::caption(lang.settings_effects_parallax_desc())
                                    .width(Length::Fill),
                            ),
                    ),
            );

        if self.config.parallax_enabled {
            body = body
                .push(widget::text::caption(format!(
                    "{}: {}%",
                    lang.settings_effects_strength(),
                    (self.config.parallax_strength * 100.0).round() as u32
                )))
                .push(widget::slider(
                    10u32..=100u32,
                    (self.config.parallax_strength * 100.0).round() as u32,
                    |value| Message::SetParallaxStrength(value as f32 / 100.0),
                ))
                .push(widget::text::caption(format!(
                    "{}: {}",
                    lang.settings_effects_depth_status_label(),
                    lang.settings_effects_depth_status(self.effects_depth_status)
                )));

            let mut model_card = widget::column::with_capacity(6)
                .spacing(8)
                .push(widget::text::body(lang.settings_effects_model_title()))
                .push(widget::text::caption(lang.settings_effects_model_desc()))
                .push(widget::text::caption(format!(
                    "{}: {}",
                    lang.settings_effects_model_path(),
                    crate::depth::model_path().display()
                )));

            match &self.effects_model_status {
                EffectsModelStatus::Absent => {
                    model_card = model_card
                        .push(widget::text::caption(lang.settings_effects_model_absent()))
                        .push(
                            widget::button::suggested(lang.settings_effects_model_download())
                                .leading_icon(widget::icon::from_name("folder-download-symbolic"))
                                .on_press(Message::DownloadDepthModel),
                        );
                }
                EffectsModelStatus::Downloading {
                    downloaded,
                    total,
                    percent,
                } => {
                    let bar: Element<'_, Message> = if *percent > 0.0 {
                        widget::progress_bar::determinate_linear(*percent)
                            .width(Length::Fill)
                            .into()
                    } else {
                        widget::progress_bar::indeterminate_linear()
                            .width(Length::Fill)
                            .into()
                    };
                    let mb_down = *downloaded as f64 / (1024.0 * 1024.0);
                    let progress_text = if let Some(total) = total {
                        let mb_total = *total as f64 / (1024.0 * 1024.0);
                        format!(
                            "{}: {:.1} / {:.1} MB ({:.0}%)",
                            lang.settings_effects_model_downloading(),
                            mb_down,
                            mb_total,
                            percent * 100.0
                        )
                    } else {
                        format!(
                            "{}: {:.1} MB",
                            lang.settings_effects_model_downloading(),
                            mb_down
                        )
                    };
                    model_card = model_card.push(bar).push(
                        widget::row::with_capacity(2)
                            .spacing(8)
                            .align_y(Alignment::Center)
                            .push(widget::text::caption(progress_text))
                            .push(
                                widget::button::standard(lang.settings_effects_model_cancel())
                                    .on_press(Message::CancelDepthModelDownload),
                            ),
                    );
                }
                EffectsModelStatus::Ready => {
                    let size = crate::depth::model_size().unwrap_or(0) as f64 / (1024.0 * 1024.0);
                    model_card = model_card
                        .push(widget::text::caption(format!(
                            "{}: {:.1} MB",
                            lang.settings_effects_model_ready(),
                            size
                        )))
                        .push(
                            widget::button::standard(lang.settings_effects_model_delete())
                                .leading_icon(widget::icon::from_name("edit-delete-symbolic"))
                                .on_press(Message::DeleteDepthModel),
                        );
                }
                EffectsModelStatus::Failed(error) => {
                    model_card = model_card
                        .push(widget::text::caption(format!(
                            "{}: {}",
                            lang.settings_effects_model_failed(),
                            error
                        )))
                        .push(
                            widget::row::with_capacity(2)
                                .spacing(8)
                                .align_y(Alignment::Center)
                                .push(
                                    widget::button::suggested(lang.settings_effects_model_retry())
                                        .on_press(Message::DownloadDepthModel),
                                )
                                .push(
                                    widget::button::standard(lang.settings_effects_model_delete())
                                        .on_press(Message::DeleteDepthModel),
                                ),
                        );
                }
            }

            body = body.push(
                widget::container(model_card)
                    .class(cosmic::theme::Container::Card)
                    .padding(12)
                    .width(Length::Fill),
            );
        }

        self.effects_card(
            lang.settings_effects_parallax(),
            "image-x-generic-symbolic",
            body.into(),
        )
    }
}
