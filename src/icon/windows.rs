#![cfg(target_os = "windows")]

use super::Icon;

struct Os;

impl super::IconInterface for Os {
    fn get_default_icon() -> Icon {
        todo!()
    }

    fn get_icon_for_file<P: AsRef<std::path::Path>>(path: P) -> Option<Icon> {
        todo!()
    }

    fn get_icon_for_url(url: &str) -> Option<Icon> {
        todo!()
    }
}
