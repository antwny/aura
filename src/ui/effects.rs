use crate::app::{AuraApp, EffectsModelStatus, EffectsParallaxStatus, Message};
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

const PARALLAX_PRESETS: [f32; 3] = [0.25, 0.55, 0.85];

fn parallax_preset_active(strength: f32, preset: f32) -> bool {
    (strength - preset).abs() < 0.001
}

fn effects_choice_chip<'a>(
    label: impl Into<std::borrow::Cow<'a, str>>,
    active: bool,
    message: Message,
) -> Element<'a, Message> {
    let button = if active {
        widget::button::suggested(label)
            .leading_icon(widget::icon::from_name("emblem-ok-symbolic"))
    } else {
        widget::button::standard(label)
    };
    button.on_press(message).into()
}

fn disclosure_icon(expanded: bool) -> &'static str {
    if expanded {
        "pan-up-symbolic"
    } else {
        "pan-down-symbolic"
    }
}

fn parallax_status_icon(status: &EffectsParallaxStatus) -> &'static str {
    match status {
        EffectsParallaxStatus::Ready => "emblem-ok-symbolic",
        EffectsParallaxStatus::Paused => "media-playback-pause-symbolic",
        EffectsParallaxStatus::Generating => "view-refresh-symbolic",
        EffectsParallaxStatus::Disabled => "media-playback-stop-symbolic",
        EffectsParallaxStatus::Unsupported
        | EffectsParallaxStatus::NoImage
        | EffectsParallaxStatus::ModelMissing
        | EffectsParallaxStatus::Failed => "dialog-warning-symbolic",
    }
}

fn picker_labels(
    lang: crate::i18n::Language,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
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
            .push(self.view_parallax_card())
            .push(self.view_spectrum_card())
            .push(self.view_effects_renderer_card());

        widget::scrollable(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn effects_card<'a>(
        &self,
        title: &str,
        icon: &'static str,
        trailing: Element<'a, Message>,
        body: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let compact = self.window_width < 620.0;
        widget::container(
            widget::column::with_capacity(2)
                .spacing(14)
                .padding(if compact { 12 } else { 18 })
                .width(Length::Fill)
                .push(
                    widget::row::with_capacity(3)
                        .spacing(10)
                        .align_y(Alignment::Center)
                        .push(widget::icon::from_name(icon))
                        .push(widget::text::title3(title.to_string()).width(Length::Fill))
                        .push(trailing),
                )
                .push(body),
        )
        .width(Length::Fill)
        .class(cosmic::theme::Container::Card)
        .into()
    }

    fn view_effects_renderer_card(&self) -> Element<'_, Message> {
        let lang = self.language;
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

        let mut body = widget::column::with_capacity(4)
            .spacing(8)
            .width(Length::Fill)
            .push(
                widget::text::caption(format!(
                    "{}: {}",
                    lang.settings_effects_renderer_status(),
                    renderer_status
                ))
                .width(Length::Fill),
            );

        if self.effects_diagnostics_expanded {
            body = body
                .push(widget::divider::horizontal::light())
                .push(flex_row(
                    vec![
                        widget::button::standard(lang.settings_effects_restart())
                            .leading_icon(widget::icon::from_name("view-refresh-symbolic"))
                            .on_press(Message::RestartEffects)
                            .into(),
                        widget::button::text(lang.settings_effects_open_log())
                            .leading_icon(widget::icon::from_name("text-x-generic-symbolic"))
                            .on_press(Message::OpenEffectsLog)
                            .into(),
                    ],
                    8,
                ))
                .push(
                    widget::text::caption(format!(
                        "{}: {}",
                        lang.settings_effects_open_log(),
                        crate::effects::log_path().display()
                    ))
                    .width(Length::Fill),
                );

            if !crate::effects::renderer_supported() {
                body = body.push(
                    widget::text::caption(lang.settings_effects_unsupported())
                        .width(Length::Fill),
                );
            }
        }

        self.effects_card(
            lang.settings_effects_diagnostics(),
            "utilities-system-monitor-symbolic",
            widget::button::text(lang.settings_effects_advanced())
                .trailing_icon(widget::icon::from_name(disclosure_icon(
                    self.effects_diagnostics_expanded,
                )))
                .on_press(Message::ToggleEffectsDiagnostics)
                .into(),
            body.into(),
        )
    }

    fn view_spectrum_card(&self) -> Element<'_, Message> {
        let lang = self.language;
        let mut body = widget::column::with_capacity(24)
            .spacing(12)
            .width(Length::Fill)
            .push(
                widget::text::caption(lang.settings_effects_spectrum_desc())
                    .width(Length::Fill),
            );

        if !self.config.spectrum_enabled {
            return self.effects_card(
                lang.settings_effects_spectrum(),
                "audio-x-generic-symbolic",
                widget::toggler(self.config.spectrum_enabled)
                    .on_toggle(Message::ToggleSpectrum)
                    .into(),
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
            .push(widget::text::body(lang.settings_effects_spectrum_appearance()))
            .push(widget::text::caption(lang.settings_effects_color_mode()))
            .push(flex_row(
                vec![
                    effects_choice_chip(
                        lang.settings_effects_color_auto(),
                        auto,
                        Message::SetSpectrumColorMode(SpectrumColorMode::Auto),
                    ),
                    effects_choice_chip(
                        lang.settings_effects_color_manual(),
                        !auto,
                        Message::SetSpectrumColorMode(SpectrumColorMode::Manual),
                    ),
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
                        widget::column::with_capacity(2)
                            .spacing(3)
                            .width(Length::Fill)
                            .push(widget::text::body(lang.settings_effects_glow()))
                            .push(
                                widget::text::caption(lang.settings_effects_glow_desc())
                                    .width(Length::Fill),
                            ),
                    )
                    .push(
                        widget::toggler(self.config.spectrum_glow)
                            .on_toggle(Message::ToggleSpectrumGlow),
                    ),
            )
            .push(widget::divider::horizontal::light())
            .push(widget::text::body(lang.settings_effects_spectrum_geometry()))
            .push(widget::text::caption(lang.settings_effects_position()))
            .push(flex_row(
                vec![
                    effects_choice_chip(
                        lang.settings_effects_position_bottom(),
                        self.config.spectrum_position == SpectrumPosition::Bottom,
                        Message::SetSpectrumPosition(SpectrumPosition::Bottom),
                    ),
                    effects_choice_chip(
                        lang.settings_effects_position_top(),
                        self.config.spectrum_position == SpectrumPosition::Top,
                        Message::SetSpectrumPosition(SpectrumPosition::Top),
                    ),
                ],
                8,
            ))
            .push(widget::text::caption(lang.settings_effects_style()))
            .push(flex_row(
                vec![
                    effects_choice_chip(
                        lang.settings_effects_style_bars(),
                        self.config.spectrum_style == SpectrumStyle::Bars,
                        Message::SetSpectrumStyle(SpectrumStyle::Bars),
                    ),
                    effects_choice_chip(
                        lang.settings_effects_style_wave(),
                        self.config.spectrum_style == SpectrumStyle::Wave,
                        Message::SetSpectrumStyle(SpectrumStyle::Wave),
                    ),
                    effects_choice_chip(
                        lang.settings_effects_style_dots(),
                        self.config.spectrum_style == SpectrumStyle::Dots,
                        Message::SetSpectrumStyle(SpectrumStyle::Dots),
                    ),
                    effects_choice_chip(
                        lang.settings_effects_style_mirror(),
                        self.config.spectrum_style == SpectrumStyle::Mirror,
                        Message::SetSpectrumStyle(SpectrumStyle::Mirror),
                    ),
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

        body = body.push(
            widget::text::caption(lang.settings_effects_spectrum_cava_hint())
                .width(Length::Fill),
        );

        self.effects_card(
            lang.settings_effects_spectrum(),
            "audio-x-generic-symbolic",
            widget::toggler(self.config.spectrum_enabled)
                .on_toggle(Message::ToggleSpectrum)
                .into(),
            body.into(),
        )
    }

    fn height_preset(&self, label: &str, height: u32) -> Element<'_, Message> {
        effects_choice_chip(
            format!("{label} · {height}%"),
            self.config.spectrum_height_pct == height,
            Message::SetSpectrumHeight(height),
        )
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
        let mut body = widget::column::with_capacity(12)
            .spacing(12)
            .width(Length::Fill)
            .push(
                widget::text::caption(lang.settings_effects_parallax_desc())
                    .width(Length::Fill),
            );

        if self.config.parallax_enabled {
            let strength_percent =
                ((self.config.parallax_strength * 100.0).round() as u32).clamp(10, 100);
            let presets = PARALLAX_PRESETS
                .iter()
                .enumerate()
                .map(|(index, &strength)| {
                    effects_choice_chip(
                        format!(
                            "{} · {:.0}%",
                            lang.settings_effects_parallax_preset(index as u8),
                            strength * 100.0
                        ),
                        parallax_preset_active(self.config.parallax_strength, strength),
                        Message::SetParallaxStrength(strength),
                    )
                })
                .collect();
            let pause_supported = self.occlusion_watcher_supported();
            let pause_toggle = widget::toggler(self.config.parallax_auto_pause);
            let pause_toggle = if pause_supported {
                pause_toggle.on_toggle(Message::ToggleParallaxAutoPause)
            } else {
                pause_toggle
            };

            body = body
                .push(widget::text::body(format!(
                    "{}: {}%",
                    lang.settings_effects_strength(),
                    strength_percent
                )))
                .push(widget::slider(
                    10u32..=100u32,
                    strength_percent,
                    |value| Message::SetParallaxStrength(value as f32 / 100.0),
                ))
                .push(widget::text::caption(lang.settings_effects_parallax_presets()))
                .push(flex_row(presets, 8))
                .push(widget::divider::horizontal::light())
                .push(
                    widget::row::with_capacity(2)
                        .spacing(14)
                        .align_y(Alignment::Center)
                        .push(
                            widget::text::body(lang.settings_effects_parallax_auto_pause())
                                .width(Length::Fill),
                        )
                        .push(pause_toggle),
                )
                .push(
                    widget::text::caption(lang.settings_effects_parallax_auto_pause_desc())
                        .width(Length::Fill),
                );

            if !pause_supported {
                body = body.push(
                    widget::row::with_capacity(2)
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(widget::icon::from_name("dialog-warning-symbolic").size(16))
                        .push(
                            widget::text::caption(lang.settings_effects_parallax_pause_unavailable())
                                .width(Length::Fill),
                        ),
                );
            }
        } else {
            body = body.push(
                widget::text::caption(
                    lang.settings_effects_parallax_status(EffectsParallaxStatus::Disabled),
                )
                .width(Length::Fill),
            );
        }

        // Per-output readiness is independent of whether the renderer process exists.
        if !self.outputs.is_empty() {
            let mut statuses = widget::column::with_capacity(self.outputs.len() + 1)
                .spacing(6)
                .width(Length::Fill)
                .push(widget::text::body(lang.settings_effects_depth_status_label()));
            for output in &self.outputs {
                let status = self.parallax_status_for(&output.name);
                statuses = statuses.push(
                    widget::row::with_capacity(2)
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(widget::icon::from_name(parallax_status_icon(&status)).size(16))
                        .push(
                            widget::text::caption(format!(
                                "{}: {}",
                                output.name,
                                lang.settings_effects_parallax_status(status)
                            ))
                            .width(Length::Fill),
                        ),
                );
            }
            body = body.push(statuses);
        }

        if self.config.parallax_enabled {
            body = body
                .push(
                    widget::text::caption(lang.settings_effects_parallax_resources())
                        .width(Length::Fill),
                )
                .push(widget::divider::horizontal::light())
                // Required model actions must not depend on either disclosure state.
                .push(self.view_effects_depth_model());
        }

        self.effects_card(
            lang.settings_effects_parallax(),
            "image-x-generic-symbolic",
            widget::toggler(self.config.parallax_enabled)
                .on_toggle(Message::ToggleParallax)
                .into(),
            body.into(),
        )
    }

    fn view_effects_depth_model(&self) -> Element<'_, Message> {
        let lang = self.language;
        let mut body = widget::column::with_capacity(6)
            .spacing(8)
            .width(Length::Fill);

        if !matches!(&self.effects_model_status, EffectsModelStatus::Ready) {
            body = body
                .push(widget::text::body(lang.settings_effects_model_title()))
                .push(
                    widget::text::caption(lang.settings_effects_model_desc())
                        .width(Length::Fill),
                );
        }

        match &self.effects_model_status {
            EffectsModelStatus::Absent => {
                body = body
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
                body = body
                    .push(widget::text::caption(progress_text).width(Length::Fill))
                    .push(bar)
                    .push(
                        widget::button::standard(lang.settings_effects_model_cancel())
                            .on_press(Message::CancelDepthModelDownload),
                    );
            }
            EffectsModelStatus::Ready => {
                let ready_text = crate::depth::model_size()
                    .map(|size| {
                        format!(
                            "{} · {:.1} MB",
                            lang.settings_effects_model_ready(),
                            size as f64 / (1024.0 * 1024.0)
                        )
                    })
                    .unwrap_or_else(|| lang.settings_effects_model_ready().to_string());
                body = body.push(flex_row(
                    vec![
                        widget::row::with_capacity(2)
                            .spacing(6)
                            .align_y(Alignment::Center)
                            .push(widget::icon::from_name("emblem-ok-symbolic").size(16))
                            .push(widget::text::caption(ready_text))
                            .into(),
                        widget::button::text(lang.settings_effects_model_details())
                            .trailing_icon(widget::icon::from_name(disclosure_icon(
                                self.effects_model_details_expanded,
                            )))
                            .on_press(Message::ToggleDepthModelDetails)
                            .into(),
                    ],
                    8,
                ));

                if self.effects_model_details_expanded {
                    body = body
                        .push(
                            widget::text::caption(lang.settings_effects_model_desc())
                                .width(Length::Fill),
                        )
                        .push(
                            widget::text::caption(format!(
                                "{}: {}",
                                lang.settings_effects_model_path(),
                                crate::depth::model_path().display()
                            ))
                            .width(Length::Fill),
                        )
                        .push(
                            widget::button::text(lang.settings_effects_model_delete())
                                .leading_icon(widget::icon::from_name("edit-delete-symbolic"))
                                .on_press(Message::DeleteDepthModel),
                        );
                }
            }
            EffectsModelStatus::Failed(error) => {
                body = body
                    .push(
                        widget::text::caption(format!(
                            "{}: {}",
                            lang.settings_effects_model_failed(),
                            error
                        ))
                        .width(Length::Fill),
                    )
                    .push(flex_row(
                        vec![
                            widget::button::suggested(lang.settings_effects_model_retry())
                                .leading_icon(widget::icon::from_name("view-refresh-symbolic"))
                                .on_press(Message::DownloadDepthModel)
                                .into(),
                            widget::button::text(lang.settings_effects_model_delete())
                                .leading_icon(widget::icon::from_name("edit-delete-symbolic"))
                                .on_press(Message::DeleteDepthModel)
                                .into(),
                        ],
                        8,
                    ));
            }
        }

        body.into()
    }
}

#[cfg(test)]
mod tests {
    use super::{parallax_preset_active, PARALLAX_PRESETS};

    #[test]
    fn each_parallax_preset_selects_only_its_own_chip() {
        assert_eq!(PARALLAX_PRESETS, [0.25, 0.55, 0.85]);
        for (selected, &strength) in PARALLAX_PRESETS.iter().enumerate() {
            for (index, &preset) in PARALLAX_PRESETS.iter().enumerate() {
                assert_eq!(parallax_preset_active(strength, preset), index == selected);
            }
        }
    }

    #[test]
    fn parallax_preset_tolerance_is_strict() {
        assert!(parallax_preset_active(0.2505, 0.25));
        assert!(parallax_preset_active(0.2495, 0.25));
        assert!(!parallax_preset_active(0.252, 0.25));
        assert!(!parallax_preset_active(0.0, 0.001));
    }

    #[test]
    fn custom_and_non_finite_strengths_do_not_select_a_preset() {
        for strength in [0.4, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(PARALLAX_PRESETS
                .iter()
                .all(|&preset| !parallax_preset_active(strength, preset)));
        }
    }
}
