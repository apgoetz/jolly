use crate::{log, ui};
use serde;

#[derive(serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct Settings {
    pub ui: ui::UISettings,
    pub log: log::LogSettings,
}
