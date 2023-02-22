use iced::{Application, Settings};
use jolly::{config, error, Jolly};

pub fn main() -> Result<(), error::Error> {
    let config = config::Config::load();

    let mut settings = Settings::default();
    settings.window.size = (
        config.settings.ui.width,
        config.settings.ui.search.starting_height(),
    );
    settings.window.decorations = false;
    settings.window.visible = false;
    settings.default_text_size = config.settings.ui.common.text_size();
    settings.flags = config;

    Jolly::run(settings).map_err(error::Error::IcedError)
}
