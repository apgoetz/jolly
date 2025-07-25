#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use iced::{Application, Settings, Size};
use jolly::{cli, config, Jolly};
use std::process::ExitCode;
use std::time::Instant;

pub fn main() -> ExitCode {
    let custom_config = match cli::parse_args(std::env::args()) {
        Ok(c) => c.config,
        Err(e) => return e,
    };

    let now = Instant::now();

    let mut config = if let Some(path) = custom_config {
        config::Config::custom_load(path)
    } else {
        config::Config::load()
    };

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
    let mut wsettings = iced::window::Settings::default();
    wsettings.size = Size{ width: config.settings.ui.width as f32, height: config.settings.ui.search.starting_height() as f32 };
    wsettings.decorations = false;
    wsettings.visible = false;
    
    settings.default_text_size = config.settings.ui.common.text_size().into();

    //probably missing something here about setting windows settings and subscriptions???
    iced::application("JOLLY BUT YOU ARENT SUPPOSED TO SEE THIS", Jolly::update, Jolly::view)
    .subscription(Jolly::subscription)
    .settings(settings)
    .window(wsettings)
    .theme(Jolly::theme)
    .run_with(||Jolly::new(config))
    .map(|_| ExitCode::SUCCESS)
        .unwrap_or(ExitCode::FAILURE)
}
