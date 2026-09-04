mod app;
mod cli;
mod config;
mod engine;
mod i18n;
mod scanner;
mod theme;
mod tray;

fn main() -> cosmic::iced::Result {
    let args: Vec<String> = std::env::args().collect();
    if cli::handle_cli(&args) {
        return Ok(());
    }

    let settings = cosmic::app::Settings::default()
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(840.0)
                .min_height(580.0),
        )
        .exit_on_close(false);

    cosmic::app::run::<app::AuraApp>(settings, ())
}
