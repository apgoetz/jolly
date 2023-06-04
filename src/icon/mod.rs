// contains logic for loading icons for entries
//
// different implementations for macOs, windows, and linux. (linux and
// BSDs assumed to use freedesktop compatible icon standards)

use std::collections::HashMap;

mod linux_and_friends;
mod macos;
mod windows;

#[cfg(target_os = "macos")]
use macos::Os;

#[cfg(all(unix, not(target_os = "macos")))]
use linux_and_friends::Os;

#[cfg(target_os = "windows")]
use windows::Os;

// defines functions that must be implemented for every operating
// system in order implement icons in jolly
trait IconInterface {
    // default icon to use if icon cannot be loaded.
    // must be infallible
    // the output is cached by icon module
    fn get_default_icon() -> Icon;

    // icon that would be used for a path that must exist
    // path is guaranteed to be already canonicalized
    fn get_icon_for_file<P: AsRef<std::path::Path>>(path: P) -> Option<Icon>;

    // icon to use for a specific url or protocol handler.
    fn get_icon_for_url(url: &str) -> Option<Icon>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Icon {
    width: u32,
    height: u32,
    // 'static kind of a hack for now. iced required 'static lifetime
    // on image bytes, so we leak icons as we look them up (caching
    // them, of course, to prevent duplicates).  iced also assumes
    // 8bit RGBA pixel data, but nothing checks this.
    bytes: &'static [u8],
}

impl Icon {
    // make a new icon by leaking a copy of bytes in the image data
    fn new(width: u32, height: u32, bytes: &[u8]) -> Self {
        let bytes = Box::new(bytes.to_owned());
        Self {
            width,
            height,
            bytes: Box::leak(bytes),
        }
    }
}

impl From<Icon> for iced::widget::image::Image {
    fn from(icon: Icon) -> iced::widget::image::Image {
        let handle = iced::widget::image::Handle::from_pixels(icon.width, icon.height, icon.bytes);
        iced::widget::image::Image::new(handle)
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum IconType {
    // render using icon for protocol of url
    Url(String),
    // render using icon for path
    File(std::path::PathBuf),
}

impl IconType {
    fn load_icon<I: IconInterface>(self) -> Icon {
        let icon = match self {
            IconType::Url(u) => I::get_icon_for_url(&u),
            IconType::File(p) => {
                if p.exists() {
                    if let Ok(p) = p.canonicalize() {
                        I::get_icon_for_file(p)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };
        icon.unwrap_or(default_icon())
    }
}

impl From<IconType> for Icon {
    fn from(it: IconType) -> Icon {
        it.load_icon::<Os>()
    }
}

pub fn default_icon() -> Icon {
    lazy_static::lazy_static! {
        static ref DEFAULT_ICON: Icon = Os::get_default_icon();
    }
    *DEFAULT_ICON
}
use crate::Message;
use iced_native::futures::channel::mpsc;

#[derive(Default)]
pub struct IconCache {
    cmd: Option<std::sync::mpsc::Sender<IconType>>,
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
        // if the key is the cache, either we have the icon or it has already been scheduled. either way, send it.
        if let Some(icon) = self.cache.get(it) {
            return *icon;
        }

        // if we have a reference the iconwork command channel, then we kick off a request to lookup the new icontype
        if let Some(cmd) = &self.cmd {
            cmd.send(it.clone())
                .expect("Could not send new icon lookup command");
            self.cache.insert(it.clone(), None);
        }

        // at this point, we know we had a cache miss
        None
    }

    pub fn add_icon(&mut self, it: IconType, i: Icon) {
        self.cache.insert(it, Some(i));
    }

    pub fn set_cmd(&mut self, cmd: std::sync::mpsc::Sender<IconType>) {
        self.cmd = Some(cmd);
    }
}

//create stream that satisfies the needs of iced_native::subscription::run
// TODO: add tests
pub fn icon_worker() -> mpsc::Receiver<Message> {
    // todo: fix magic for channel size
    let (mut output, sub_stream) = mpsc::channel(100);

    std::thread::spawn(move || {
        let (input, command_stream) = std::sync::mpsc::channel();

        // send the application a channel to provide us icon work
        // TODO: implement error checking if we cant send
        output
            .try_send(Message::StartedIconWorker(input))
            .expect("Could not send iconworker back to application");

        loop {
            let icontype = match command_stream.recv() {
                Ok(i) => i,
                _ => break,
            };

            use std::thread::sleep;
            use std::time::Duration;

            sleep(Duration::new(1, 0));

            // todo: handle error
            output
                .try_send(Message::IconReceived(icontype.clone(), icontype.into()))
                .expect("Could not send icon back  application");
        }
    });
    sub_stream
}

#[cfg(test)]
mod tests {
    use super::{IconInterface, Os};

    fn iconlike(icon: super::Icon, err_msg: &str) {
        let num_pixels = icon.width * icon.height;

        assert!(num_pixels > 0, "zero pixels: {}", err_msg);
        assert_eq!(
            (num_pixels * 4) as usize,
            icon.bytes.len(),
            "incorrect buffer size: {}",
            err_msg
        )
    }

    #[test]
    fn default_icon_is_iconlike() {
        // confirm image is square and bytes could be 8bit RGBA
        iconlike(Os::get_default_icon(), "for default icon");
    }

    #[test]
    fn executable_is_iconlike() {
        let cur_exe = std::env::current_exe().unwrap();
        iconlike(
            Os::get_icon_for_file(&cur_exe).unwrap(),
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
            let icon = Os::get_icon_for_url(url)
                .expect(&format!(r#"could not load icon for url "{}""#, url));

            iconlike(icon, &format!("for common url {}", url));
        }
    }

    #[test]
    fn paths_are_canonicalized() {
        struct MockIcon;

        impl IconInterface for MockIcon {
            fn get_default_icon() -> crate::icon::Icon {
                panic!("expected file, not default")
            }

            fn get_icon_for_file<P: AsRef<std::path::Path>>(path: P) -> Option<crate::icon::Icon> {
                let path = path.as_ref();
                assert!(path.as_os_str() == path.canonicalize().unwrap().as_os_str());
                Some(super::Icon {
                    width: 1,
                    height: 1,
                    bytes: &[1, 1, 1, 1],
                })
            }

            fn get_icon_for_url(_url: &str) -> Option<crate::icon::Icon> {
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
        icon_type.load_icon::<MockIcon>();
    }
}
