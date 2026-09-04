mod app;
mod config;
mod engine;
mod scanner;
mod theme;

fn main() -> cosmic::iced::Result {
    let settings = cosmic::app::Settings::default()
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(780.0)
                .min_height(520.0),
        );

    cosmic::app::run::<app::AuraApp>(settings, ())
}
