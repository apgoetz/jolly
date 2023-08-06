#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use iced::{Application, Settings};
use jolly::{config, error, Jolly};
use std::time::Instant;

pub fn main() -> Result<(), error::Error> {
    let now = Instant::now();

    let mut config = config::Config::load();

    let elapsed = now.elapsed();

    // if we could not initialize the logger, we set the store to
    // error, so the ui shows the issue
    if let Err(e) = config.settings.log.init_logger() {
        config.store = Err(e);
    }

    if let Ok(s) = &config.store {
        ::log::debug!(
            "Loaded {} entries in {:.6} sec",
            s.len(),
            elapsed.as_secs_f32()
        );
    }

    let mut settings = Settings::default();
    settings.window.size = (
        config.settings.ui.width,
        config.settings.ui.search.starting_height(),
    );
    settings.window.decorations = false;
    settings.window.visible = false;
    settings.default_text_size = config.settings.ui.common.text_size().into();
    settings.flags = config;

    Jolly::run(settings).map_err(error::Error::IcedError)
}
