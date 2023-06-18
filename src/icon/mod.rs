// contains logic for loading icons for entries
//
// different implementations for macOs, windows, and linux. (linux and
// BSDs assumed to use freedesktop compatible icon standards)

use std::collections::HashMap;
use std::hash::Hash;

mod linux_and_friends;
mod macos;
mod windows;

#[cfg(target_os = "macos")]
pub use macos::Os as IconSettings;

#[cfg(all(unix, not(target_os = "macos")))]
pub use linux_and_friends::Os as IconSettings;

#[cfg(target_os = "windows")]
pub use windows::Os as IconSettings;

// defines functions that must be implemented for every operating
// system in order implement icons in jolly
trait IconInterface {
    // default icon to use if icon cannot be loaded.
    // must be infallible
    // the output is cached by icon module, so it should not be used be other logic
    fn get_default_icon(&self) -> Icon;

    // icon that would be used for a path that must exist
    // path is guaranteed to be already canonicalized
    fn get_icon_for_file<P: AsRef<std::path::Path>>(&self, path: P) -> Option<Icon>;

    // icon to use for a specific url or protocol handler.
    fn get_icon_for_url(&self, url: &str) -> Option<Icon>;

    // provided method: uses icon interfaces to turn icontype into icon
    fn load_icon(&self, itype: IconType) -> Icon {
        let icon = match itype {
            IconType::Url(u) => self.get_icon_for_url(u.as_str()),
            IconType::File(p) => {
                if p.exists() {
                    if let Ok(p) = p.canonicalize() {
                        self.get_icon_for_file(p)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            IconType::CustomIcon(p) => Some(Icon::from_path(p)),
        };
        icon.unwrap_or(self.get_default_icon())
    }
}

// sufficient for now, until we implement SVG support
pub type Icon = iced::widget::image::Handle;

// represents the necessary information in an entry to look up an icon
// type. Importantly, url based entries are assumed to have the same
// icon if they have the same protocol (for example, all web links)
#[derive(Debug, Clone)]
pub enum IconType {
    // render using icon for protocol of url
    Url(url::Url),
    // render using icon for path
    File(std::path::PathBuf),
    // override "normal" icon and use icon from this path
    CustomIcon(std::path::PathBuf),
}

impl Hash for IconType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            IconType::Url(u) => u.scheme().hash(state),
            IconType::File(p) => p.hash(state),
            IconType::CustomIcon(p) => p.hash(state),
        }
    }
}

impl Eq for IconType {}
impl PartialEq for IconType {
    fn eq(&self, other: &Self) -> bool {
        match self {
            IconType::Url(s) => {
                if let IconType::Url(o) = other {
                    s.scheme() == o.scheme()
                } else {
                    false
                }
            }
            IconType::File(s) => {
                if let IconType::File(o) = other {
                    s == o
                } else {
                    false
                }
            }
            IconType::CustomIcon(s) => {
                if let IconType::CustomIcon(o) = other {
                    s == o
                } else {
                    false
                }
            }
        }
    }
}

pub fn default_icon(is: &IconSettings) -> Icon {
    use once_cell::sync::OnceCell;
    static DEFAULT_ICON: OnceCell<Icon> = OnceCell::new();

    DEFAULT_ICON.get_or_init(|| is.get_default_icon()).clone()
}

use crate::Message;
use iced_native::futures::channel::mpsc;

// represents an icon cache that can look up icons in a deferred worker thread
#[derive(Default)]
pub struct IconCache {
    cmd: Option<std::sync::mpsc::Sender<IconCommand>>,
    cache: HashMap<IconType, Option<Icon>>,
}

impl IconCache {
    pub fn new() -> Self {
        Self {
            cmd: None,
            cache: HashMap::new(),
        }
    }

    pub fn get(&mut self, it: &IconType) -> Option<Icon> {
        // if the key is the cache, either we have the icon or it has
        // already been scheduled. either way, send it.
        if let Some(icon) = self.cache.get(it) {
            return icon.clone();
        }

        // if we have a reference the iconwork command channel, then
        // we kick off a request to lookup the new icontype
        if let Some(cmd) = &self.cmd {
            cmd.send(IconCommand::LoadIcon(it.clone()))
                .expect("Could not send new icon lookup command");
            self.cache.insert(it.clone(), None);
        }

        // at this point, we know we had a cache miss
        None
    }

    pub fn add_icon(&mut self, it: IconType, i: Icon) {
        self.cache.insert(it, Some(i));
    }

    pub fn set_cmd(&mut self, cmd: std::sync::mpsc::Sender<IconCommand>) {
        self.cmd = Some(cmd);
    }
}

#[derive(Debug)]
pub enum IconCommand {
    LoadSettings(IconSettings),
    LoadIcon(IconType),
}

//create stream that satisfies the needs of iced_native::subscription::run
// TODO: add tests
pub fn icon_worker() -> mpsc::Receiver<Message> {
    use std::time::Instant;

    // todo: fix magic for channel size
    let (mut output, sub_stream) = mpsc::channel(100);

    std::thread::spawn(move || {
        let (input, command_stream) = std::sync::mpsc::channel();

        // send the application a channel to provide us icon work
        // TODO: implement error checking if we cant send
        output
            .try_send(Message::StartedIconWorker(input))
            .expect("Could not send iconworker back to application");

        let command = match command_stream.recv() {
            Ok(i) => i,
            _ => return,
        };
        let settings = if let IconCommand::LoadSettings(settings) = command {
            settings
        } else {
            return;
        };

        loop {
            let command = match command_stream.recv() {
                Ok(i) => i,
                _ => break,
            };

            print!("Processing cmd {:?}", &command);
            let start = Instant::now();

            match command {
                IconCommand::LoadIcon(icontype) => {
                    // todo: handle error
                    output
                        .try_send(Message::IconReceived(
                            icontype.clone(),
                            settings.load_icon(icontype),
                        ))
                        .expect("Could not send icon back  application");
                }
                _ => break,
            }

            println!("Took {} ms", start.elapsed().as_millis())
        }
    });
    sub_stream
}

#[cfg(test)]
mod tests {
    use super::{IconInterface, IconSettings};

    fn iconlike(icon: super::Icon, err_msg: &str) {
        match icon.data() {
            iced_native::image::Data::Path(p) => {
                assert!(p.exists())
            }
            iced_native::image::Data::Bytes(bytes) => {
                assert!(bytes.len() > 0)
            }
            iced_native::image::Data::Rgba {
                width,
                height,
                pixels,
            } => {
                let num_pixels = width * height;

                assert!(num_pixels > 0, "zero pixels: {}", err_msg);
                assert_eq!(
                    (num_pixels * 4) as usize,
                    pixels.len(),
                    "incorrect buffer size: {}",
                    err_msg
                )
            }
        }
    }

    #[test]
    fn default_icon_is_iconlike() {
        iconlike(
            IconSettings::default().get_default_icon(),
            "for default icon",
        );
    }

    #[test]
    fn executable_is_iconlike() {
        let cur_exe = std::env::current_exe().unwrap();
        iconlike(
            IconSettings::default().get_icon_for_file(&cur_exe).unwrap(),
            "for current executable",
        );
    }

    #[test]
    fn commons_urls_are_iconlike() {
        // test urls that are so common, every platform should support
        // them
        let urls = [
            "http://example.com",
            "https://example.com",
            "mailto:example@example.com",
            "help:asdf",
        ];
        for url in urls.iter() {
            let icon = IconSettings::default()
                .get_icon_for_url(url)
                .expect(&format!(r#"could not load icon for url "{}""#, url));

            iconlike(icon, &format!("for common url {}", url));
        }
    }

    #[test]
    fn paths_are_canonicalized() {
        struct MockIcon;

        impl IconInterface for MockIcon {
            fn get_default_icon(&self) -> crate::icon::Icon {
                super::Icon::from_pixels(1, 1, &[1, 1, 1, 1])
            }

            fn get_icon_for_file<P: AsRef<std::path::Path>>(
                &self,
                path: P,
            ) -> Option<crate::icon::Icon> {
                let path = path.as_ref();
                assert!(path.as_os_str() == path.canonicalize().unwrap().as_os_str());
                Some(self.get_default_icon())
            }

            fn get_icon_for_url(&self, _url: &str) -> Option<crate::icon::Icon> {
                panic!("expected file, not url")
            }
        }

        use tempfile;
        let curdir = std::env::current_dir().unwrap();
        let dir = tempfile::tempdir_in(&curdir).unwrap();
        let dirname = dir.path().strip_prefix(curdir).unwrap();

        let filename = dirname.join("test.txt");

        let _file = std::fs::File::create(&filename).unwrap();

        let icon_type = super::IconType::File(filename);
        let mock = MockIcon;
        mock.load_icon(icon_type);
    }
}
