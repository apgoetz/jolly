#![cfg(target_os = "windows")]

use super::Icon;

pub struct Os;

impl super::IconInterface for Os {
    fn get_default_icon(&self) -> Icon {
        todo!()
    }

    fn get_icon_for_file<P: AsRef<std::path::Path>>(&self, path: P) -> Option<Icon> {
        todo!()
    }

    fn get_icon_for_url(&self, url: &str) -> Option<Icon> {
        todo!()
    }
}
