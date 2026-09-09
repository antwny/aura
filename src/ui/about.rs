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
            )
            .push(
                widget::text::caption(self.language.about_summary_desc())
            );

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
                    .on_press(Message::OpenPayPal)
            );

        col = col.push(logo_widget)
            .push(title_box)
            .push(widget::container(info_card))
            .push(action_buttons);

        Element::from(widget::scrollable(col).height(Length::Fill))
    }
}
