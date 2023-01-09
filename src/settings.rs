use serde;

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub ui_width: u32,
    pub ui_max_results: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ui_width: 800,
            ui_max_results: 5,
        }
    }
}
