// eventually the jolly main window logic will move here out of main
// but for now it will just hold settings.

use serde;

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct UISettings {
    pub default_text_size: u16,
    pub default_padding: u16,
    pub width: u32,
    pub max_results: u32,
}

impl UISettings {
    pub fn starting_height(&self) -> u32 {
        (self.default_text_size + 2 * self.default_padding).into()
    }
}

impl Default for UISettings {
    fn default() -> Self {
        Self {
            default_text_size: 20,
            default_padding: 10,
            width: 800,
            max_results: 5,
        }
    }
}
