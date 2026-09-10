use crate::app::{AuraApp, Message};
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    pub(crate) fn view_about(&self) -> Element<'_, Message> {
        let mut col = widget::column::with_capacity(6)
            .spacing(18)
            .padding(24)
            .align_x(Horizontal::Center)
            .width(Length::Fill);

        // Official high-resolution Kinetic A vector logo
        let logo_widget = widget::icon::from_svg_bytes(
            include_bytes!("../../resources/icons/hicolor/scalable/apps/io.github.antwny.aura.svg")
        )
        .icon()
        .size(104);

        let title_box = widget::column::with_capacity(3)
            .spacing(6)
            .align_x(Horizontal::Center)
            .push(widget::text::title1("Aura"))
            .push(widget::text::title3(self.language.about_tagline()))
            .push(widget::text::caption(self.language.about_version_info()));

        let info_card = widget::column::with_capacity(7)
            .spacing(12)
            .padding(20)
            .width(Length::Fixed(580.0))
            .push(widget::text::title3(self.language.about_details_title()))
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .push(widget::text::body(self.language.about_developer_lbl()).width(Length::Fixed(140.0)))
                    .push(widget::text::body("Antwny"))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .push(widget::text::body(self.language.about_donation_lbl()).width(Length::Fixed(140.0)))
                    .push(widget::text::body("antwnyab@gmail.com"))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .push(widget::text::body(self.language.about_architecture_lbl()).width(Length::Fixed(140.0)))
                    .push(widget::text::caption("Rust 1.95 • libcosmic • wgpu • Tokio • mpvpaper"))
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .push(widget::text::body(self.language.about_license_lbl()).width(Length::Fixed(140.0)))
                    .push(widget::text::body("GNU General Public License v3.0 (GPL-3.0)"))
            );

        // Software Updates Card
        let update_card = {
            let mut u_col = widget::column::with_capacity(4).spacing(10).padding(20).width(Length::Fixed(580.0));
            u_col = u_col.push(widget::text::title3(self.language.about_updates_title()));

            if crate::online::updater::is_flatpak() {
                u_col = u_col.push(widget::text::caption(self.language.about_flatpak_managed()));
            } else {
                match &self.update_status {
                    crate::app::UpdateStatus::Idle => {
                        let row = widget::row::with_capacity(2)
                            .spacing(16)
                            .align_y(Alignment::Center)
                            .push(widget::text::body(format!("v{} • {}", env!("CARGO_PKG_VERSION"), self.language.about_up_to_date())))
                            .push(
                                widget::button::standard(self.language.about_check_updates_btn())
                                    .leading_icon(widget::icon::from_name("view-refresh-symbolic"))
                                    .on_press(Message::CheckForUpdates { user_initiated: true })
                            );
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
                        let row = widget::row::with_capacity(2)
                            .spacing(16)
                            .align_y(Alignment::Center)
                            .push(widget::text::body(format!("v{} • {}", env!("CARGO_PKG_VERSION"), self.language.about_up_to_date())))
                            .push(
                                widget::button::standard(self.language.about_check_updates_btn())
                                    .leading_icon(widget::icon::from_name("view-refresh-symbolic"))
                                    .on_press(Message::CheckForUpdates { user_initiated: true })
                            );
                        u_col = u_col.push(row);
                    }
                    crate::app::UpdateStatus::Available { latest_tag, notes, download_url } => {
                        let badge_text = format!("🚀 {} {} (actual: v{})", self.language.about_update_available(), latest_tag, env!("CARGO_PKG_VERSION"));
                        let update_btn = if let Some(url) = download_url {
                            widget::button::suggested(format!("{} {}", self.language.about_update_now_btn(), latest_tag))
                                .leading_icon(widget::icon::from_name("software-update-available-symbolic"))
                                .on_press(Message::PerformGuiUpdate { download_url: url.clone(), version: latest_tag.clone() })
                        } else {
                            widget::button::standard(self.language.about_github_btn())
                                .on_press(Message::OpenGitHub)
                        };

                        let content_row = widget::row::with_capacity(2)
                            .spacing(16)
                            .align_y(Alignment::Center)
                            .push(widget::text::body(badge_text))
                            .push(update_btn);

                        u_col = u_col.push(content_row);
                        if !notes.trim().is_empty() {
                            let clean_notes = if notes.len() > 180 {
                                format!("{}...", &notes[..180])
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
                        let row = widget::row::with_capacity(2)
                            .spacing(16)
                            .align_y(Alignment::Center)
                            .push(widget::text::body(format!("🎉 ¡Aura actualizada a {}!", new_version)))
                            .push(
                                widget::button::suggested(self.language.about_restart_btn())
                                    .leading_icon(widget::icon::from_name("system-restart-symbolic"))
                                    .on_press(Message::RestartApp)
                            );
                        u_col = u_col.push(row);
                    }
                    crate::app::UpdateStatus::Error(err) => {
                        let row = widget::row::with_capacity(2)
                            .spacing(16)
                            .align_y(Alignment::Center)
                            .push(widget::text::caption(format!("Error: {}", err)))
                            .push(
                                widget::button::standard(self.language.about_check_updates_btn())
                                    .leading_icon(widget::icon::from_name("view-refresh-symbolic"))
                                    .on_press(Message::CheckForUpdates { user_initiated: true })
                            );
                        u_col = u_col.push(row);
                    }
                }
            }

            widget::container(u_col)
        };

        let action_buttons = widget::row::with_capacity(3)
            .spacing(14)
            .align_y(Alignment::Center)
            .push(
                widget::button::suggested(self.language.about_github_btn())
                    .on_press(Message::OpenGitHub)
            )
            .push(
                widget::button::standard(self.language.about_youtube_btn())
                    .on_press(Message::OpenYouTube)
            )
            .push(
                widget::button::standard(self.language.about_donate_btn())
                    .leading_icon(widget::icon::from_name("emblem-favorite-symbolic"))
                    .on_press(Message::OpenPayPal)
            );

        col = col.push(logo_widget)
            .push(title_box)
            .push(widget::container(info_card))
            .push(update_card)
            .push(action_buttons);

        Element::from(widget::scrollable(col).height(Length::Fill))
    }
}
