use crate::app::{AuraApp, Message};
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    fn about_info_row<'a>(
        &'a self,
        label: &'a str,
        value: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let value = value.into();
        if self.window_width < 560.0 {
            widget::column::with_capacity(2)
                .spacing(3)
                .width(Length::Fill)
                .push(widget::text::body(label))
                .push(value)
                .into()
        } else {
            widget::row::with_capacity(2)
                .spacing(20)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .push(widget::text::body(label).width(Length::Fixed(140.0)))
                .push(widget::container(value).width(Length::Fill))
                .into()
        }
    }

    pub(crate) fn view_about(&self) -> Element<'_, Message> {
        let compact = self.window_width < 620.0;
        let mut col = widget::column::with_capacity(6)
            .spacing(if compact { 12 } else { 18 })
            .padding(if compact { 12 } else { 24 })
            .align_x(Horizontal::Center)
            .width(Length::Fill);

        // Official high-resolution Kinetic A vector logo
        let logo_widget = widget::icon::from_svg_bytes(
            include_bytes!("../../resources/icons/hicolor/scalable/apps/io.github.antwny.aura.svg")
        )
        .icon()
        .size(if compact { 72 } else { 96 });

        let title_box = widget::column::with_capacity(3)
            .spacing(6)
            .align_x(Horizontal::Center)
            .push(widget::text::title1("Aura"))
            .push(widget::text::body(self.language.about_tagline()))
            .push(widget::text::caption(self.language.about_version_info()));

        let info_card = widget::column::with_capacity(3)
            .spacing(12)
            .padding(if compact { 12 } else { 18 })
            .width(Length::Fill)
            .max_width(560.0)
            .align_x(Horizontal::Center)
            .push(self.about_info_row(
                self.language.about_developer_lbl(),
                widget::text::body("Antwny"),
            ))
            .push(self.about_info_row(
                self.language.about_license_lbl(),
                widget::text::body("GNU General Public License v3.0 (GPL-3.0)"),
            ));

        // Software Updates Card
        let update_card = {
            let mut u_col = widget::column::with_capacity(4)
                .spacing(10)
                .padding(if compact { 12 } else { 18 })
                .width(Length::Fill)
                .max_width(560.0);
            u_col = u_col.push(widget::text::title3(self.language.about_updates_title()));

            if crate::online::updater::is_flatpak() {
                u_col = u_col.push(widget::text::caption(self.language.about_flatpak_managed()));
            } else {
                match &self.update_status {
                    crate::app::UpdateStatus::Idle => {
                        let row = widget::flex_row(vec![
                            widget::text::body(format!("v{} • {}", env!("CARGO_PKG_VERSION"), self.language.about_up_to_date())).into(),
                            widget::button::standard(self.language.about_check_updates_btn())
                                .leading_icon(widget::icon::from_name("view-refresh-symbolic"))
                                .on_press(Message::CheckForUpdates { user_initiated: true })
                                .into(),
                        ])
                            .spacing(16)
                            .align_items(Alignment::Center)
                            .width(Length::Fill);
                        u_col = u_col.push(row);
                    }
                    crate::app::UpdateStatus::Checking => {
                        let row = widget::row::with_capacity(2)
                            .spacing(12)
                            .align_y(Alignment::Center)
                            .push(widget::icon::from_name("process-working-symbolic"))
                            .push(widget::text::body(self.language.about_checking_updates()));
                        u_col = u_col.push(row);
                    }
                    crate::app::UpdateStatus::UpToDate => {
                        let row = widget::flex_row(vec![
                            widget::text::body(format!("v{} • {}", env!("CARGO_PKG_VERSION"), self.language.about_up_to_date())).into(),
                            widget::button::standard(self.language.about_check_updates_btn())
                                .leading_icon(widget::icon::from_name("view-refresh-symbolic"))
                                .on_press(Message::CheckForUpdates { user_initiated: true })
                                .into(),
                        ])
                            .spacing(16)
                            .align_items(Alignment::Center)
                            .width(Length::Fill);
                        u_col = u_col.push(row);
                    }
                    crate::app::UpdateStatus::Available { latest_tag, notes, download_url, checksum_url } => {
                        let badge_text = format!("🚀 {} {} (actual: v{})", self.language.about_update_available(), latest_tag, env!("CARGO_PKG_VERSION"));
                        let update_btn = if let (Some(url), Some(checksum_url)) = (download_url, checksum_url) {
                            widget::button::suggested(format!("{} {}", self.language.about_update_now_btn(), latest_tag))
                                .leading_icon(widget::icon::from_name("software-update-available-symbolic"))
                                .on_press(Message::PerformGuiUpdate {
                                    download_url: url.clone(),
                                    checksum_url: checksum_url.clone(),
                                    version: latest_tag.clone(),
                                })
                        } else {
                            widget::button::standard(self.language.about_github_btn())
                                .on_press(Message::OpenGitHub)
                        };

                        let content_row = widget::flex_row(vec![
                            widget::text::body(badge_text).width(Length::Fill).into(),
                            update_btn.into(),
                        ])
                        .spacing(16)
                        .align_items(Alignment::Center)
                        .width(Length::Fill);

                        u_col = u_col.push(content_row);
                        if !notes.trim().is_empty() {
                            let clean_notes = if notes.chars().count() > 180 {
                                format!("{}...", notes.chars().take(180).collect::<String>())
                            } else {
                                notes.clone()
                            };
                            u_col = u_col.push(widget::text::caption(clean_notes));
                        }
                    }
                    crate::app::UpdateStatus::Downloading => {
                        let row = widget::row::with_capacity(2)
                            .spacing(12)
                            .align_y(Alignment::Center)
                            .push(widget::icon::from_name("process-working-symbolic"))
                            .push(widget::text::body(self.language.about_updating()));
                        u_col = u_col.push(row);
                    }
                    crate::app::UpdateStatus::UpdatedSuccess { new_version } => {
                        let row = widget::flex_row(vec![
                            widget::text::body(format!("🎉 ¡Aura actualizada a {}!", new_version)).into(),
                            widget::button::suggested(self.language.about_restart_btn())
                                .leading_icon(widget::icon::from_name("system-restart-symbolic"))
                                .on_press(Message::RestartApp)
                                .into(),
                        ])
                            .spacing(16)
                            .align_items(Alignment::Center)
                            .width(Length::Fill);
                        u_col = u_col.push(row);
                    }
                    crate::app::UpdateStatus::Error(err) => {
                        let row = widget::flex_row(vec![
                            widget::text::caption(format!("Error: {}", err)).width(Length::Fill).into(),
                            widget::button::standard(self.language.about_check_updates_btn())
                                .leading_icon(widget::icon::from_name("view-refresh-symbolic"))
                                .on_press(Message::CheckForUpdates { user_initiated: true })
                                .into(),
                        ])
                            .spacing(16)
                            .align_items(Alignment::Center)
                            .width(Length::Fill);
                        u_col = u_col.push(row);
                    }
                }
            }

            widget::container(u_col)
                .width(Length::Fill)
                .max_width(560.0)
        };

        let action_buttons = widget::flex_row(vec![
                widget::button::standard(self.language.about_github_btn())
                    .leading_icon(widget::icon::from_name("web-browser-symbolic"))
                    .on_press(Message::OpenGitHub)
                    .into(),
                widget::button::standard(self.language.about_youtube_btn())
                    .leading_icon(widget::icon::from_name("media-playback-start-symbolic"))
                    .on_press(Message::OpenYouTube)
                    .into(),
                widget::button::suggested(self.language.about_donate_btn())
                    .leading_icon(widget::icon::from_name("emblem-favorite-symbolic"))
                    .on_press(Message::OpenPayPal)
                    .into(),
            ])
            .spacing(14)
            .align_items(Alignment::Center)
            .width(Length::Fill);

        col = col.push(logo_widget)
            .push(title_box)
            .push(
                widget::container(info_card)
                    .width(Length::Fill)
                    .center_x(Length::Fill),
            )
            .push(update_card)
            .push(
                widget::container(action_buttons)
                    .width(Length::Fill)
                    .max_width(560.0),
            );

        Element::from(widget::scrollable(col).height(Length::Fill))
    }
}
