// contains logic for loading icons for entries
//
// different implementations for macOs, windows, and linux. (linux and
// BSDs assumed to use freedesktop compatible icon standards)

use std::collections::HashMap;
use std::hash::Hash;
use url::Url;

use std::error;

mod linux_and_friends;
mod macos;
mod windows;

use lazy_static::lazy_static;
lazy_static! {
    static ref FALLBACK_ICON: Icon = Icon::from_pixels(1, 1, &[127, 127, 127, 255]);
}

// TODO
//
// This is a list of supported icon formats by iced_graphics.
// This is based on iced_graphics using image-rs to load images, and
// looking at what features are enabled on that package. In the future
// we may not compile support for all formats but (for now) we have a
// comprehensive list here
const SUPPORTED_ICON_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "pbm", "pam", "ppm", "pgm", "tiff", "tif", "tga", "dds",
    "bmp", "ico", "hdr", "exr", "ff", "qoi",
];

const DEFAULT_ICON_SIZE: u16 = 48; // TODO, support other icon sizes

#[cfg(target_os = "macos")]
pub use macos::Os as IconSettings;

#[cfg(all(unix, not(target_os = "macos")))]
pub use linux_and_friends::Os as IconSettings;

#[cfg(target_os = "windows")]
pub use self::windows::Os as IconSettings;

#[derive(Debug)]
struct IconError(String, Option<Box<dyn error::Error + 'static>>);

use std::fmt;
impl fmt::Display for IconError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl error::Error for IconError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.1.as_deref()
    }
}

impl<S: AsRef<str> + fmt::Display> From<S> for IconError {
    fn from(value: S) -> Self {
        Self(value.to_string(), None)
    }
}

trait Context<T> {
    fn context<S: AsRef<str> + fmt::Display>(self, msg: S) -> Result<T, IconError>;
}

impl<T, E: error::Error + 'static> Context<T> for Result<T, E> {
    fn context<S: AsRef<str> + fmt::Display>(self, msg: S) -> Result<T, IconError> {
        self.map_err(|e| IconError(msg.to_string(), Some(Box::new(e))))
    }
}

impl<T> Context<T> for Option<T> {
    fn context<S: AsRef<str> + fmt::Display>(self, msg: S) -> Result<T, IconError> {
        self.ok_or(IconError(msg.to_string(), None))
    }
}

// defines functions that must be implemented for every operating
// system in order implement icons in jolly
trait IconInterface {
    // default icon to use if icon cannot be loaded.
    // must be infallible
    // the output is cached by icon module, so it should not be used be other logic
    fn get_default_icon(&self) -> Icon;

    // icon that would be used for a path that must exist
    // path is guaranteed to be already canonicalized
    fn get_icon_for_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<Icon, IconError>;

    // icon to use for a specific url or protocol handler.
    fn get_icon_for_url(&self, url: &str) -> Result<Icon, IconError>;

    // version of get_default_icon that caches its value. One value for lifetime of application
    fn cached_default(&self) -> Icon {
        use once_cell::sync::OnceCell;
        static DEFAULT_ICON: OnceCell<Icon> = OnceCell::new();

        DEFAULT_ICON.get_or_init(|| self.get_default_icon()).clone()
    }

    // provided method: uses icon interfaces to turn icontype into icon
    fn load_icon(&self, itype: IconType) -> Icon {
        let icon = self.inner_load_icon(itype);
        icon.unwrap_or(self.cached_default())
    }

    fn inner_load_icon(&self, itype: IconType) -> Result<Icon, IconError> {
        match itype.0 {
            IconVariant::Url(u) => self.get_icon_for_url(u.as_str()),
            IconVariant::File(p) => {
                if p.exists() {
                    if let Ok(p) = p.canonicalize() {
                        self.get_icon_for_file(p)
                    } else {
                        Err("File Icon does not exist".into())
                    }
                } else {
                    Err("Cannot load icon for nonexistant file".into()) // TODO handle file type lookup by extension
                }
            }
            IconVariant::CustomIcon(p) => {
                let ext = p.extension().context("No extension on custom icon file")?;
                if SUPPORTED_ICON_EXTS
                    .iter()
                    .find(|s| ext.eq_ignore_ascii_case(s))
                    .is_some()
                {
                    Ok(Icon::from_path(p))
                } else if ext.eq_ignore_ascii_case("svg") {
                    icon_from_svg(&p)
                } else {
                    Err("is unsupported icon type".into())
                }
            }
        }
    }
}

// sufficient for now, until we implement SVG support
pub type Icon = iced::widget::image::Handle;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct IconType(IconVariant);

impl IconType {
    pub fn custom<P: AsRef<std::path::Path>>(path: P) -> Self {
        Self(IconVariant::CustomIcon(path.as_ref().into()))
    }
    pub fn url(url: Url) -> Self {
        // hack to make paths that start with disk drives not show up as URLs
        #[cfg(target_os = "windows")]
        if url.scheme().len() == 1
            && "abcdefghijklmnopqrstuvwxyz".contains(url.scheme().chars().next().unwrap())
        {
            return Self(IconVariant::File(url.as_ref().into()));
        }

        Self(IconVariant::Url(url))
    }
    pub fn file<P: AsRef<std::path::Path>>(path: P) -> Self {
        Self(IconVariant::File(path.as_ref().into()))
    }
}

// represents the necessary information in an entry to look up an icon
// type. Importantly, url based entries are assumed to have the same
// icon if they have the same protocol (for example, all web links)
#[derive(Debug, Clone)]
enum IconVariant {
    // render using icon for protocol of url
    Url(url::Url),
    // render using icon for path
    File(std::path::PathBuf),
    // override "normal" icon and use icon from this path
    CustomIcon(std::path::PathBuf),
}

impl Hash for IconVariant {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            IconVariant::Url(u) => u.scheme().hash(state),
            IconVariant::File(p) => p.hash(state),
            IconVariant::CustomIcon(p) => p.hash(state),
        }
    }
}

impl Eq for IconVariant {}
impl PartialEq for IconVariant {
    fn eq(&self, other: &Self) -> bool {
        match self {
            IconVariant::Url(s) => {
                if let IconVariant::Url(o) = other {
                    s.scheme() == o.scheme()
                } else {
                    false
                }
            }
            IconVariant::File(s) => {
                if let IconVariant::File(o) = other {
                    s == o
                } else {
                    false
                }
            }
            IconVariant::CustomIcon(s) => {
                if let IconVariant::CustomIcon(o) = other {
                    s == o
                } else {
                    false
                }
            }
        }
    }
}

pub fn default_icon(is: &IconSettings) -> Icon {
    is.cached_default()
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
        }
    });
    sub_stream
}

// convert an svg file into a pixmap
fn icon_from_svg(path: &std::path::Path) -> Result<Icon, IconError> {
    use resvg::usvg::TreeParsing;
    let svg_data = std::fs::read(path).context("could not open file")?;
    let utree = resvg::usvg::Tree::from_data(&svg_data, &Default::default())
        .context("could not parse svg")?;

    let icon_size = DEFAULT_ICON_SIZE as u32;

    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(icon_size, icon_size).context("could not create pixmap")?;

    let rtree = resvg::Tree::from_usvg(&utree);

    // we have non-square svg
    if rtree.size.width() != rtree.size.height() {
        return Err("SVG icons must be square".into());
    }

    let scalefactor = icon_size as f32 / rtree.size.width();
    let transform = resvg::tiny_skia::Transform::from_scale(scalefactor, scalefactor);

    rtree.render(transform, &mut pixmap.as_mut());

    Ok(Icon::from_pixels(
        icon_size,
        icon_size,
        pixmap.take().leak(),
    ))
}

#[cfg(test)]
mod tests {
    use super::{Icon, IconError, IconInterface, IconSettings};

    fn hash_eq_icon(icon: &Icon, ficon: &Icon) -> bool {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut ihash = DefaultHasher::new();
        let mut fhash = DefaultHasher::new();
        icon.hash(&mut ihash);
        ficon.hash(&mut fhash);
        ihash.finish() == fhash.finish()
    }

    fn iconlike(icon: Icon, err_msg: &str) {
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
        };

        assert!(
            !hash_eq_icon(&icon, &super::FALLBACK_ICON),
            "icon hash matches fallback icon, should not occur during happycase"
        );
    }

    #[test]
    fn default_icon_is_iconlike() {
        iconlike(
            IconSettings::default().get_default_icon(),
            "for default icon",
        );
    }

    // ignore on linux since exes are all detected as libraries which
    // dont have a default icon
    #[test]
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn executable_is_iconlike() {
        let cur_exe = std::env::current_exe().unwrap();

        iconlike(
            IconSettings::default().get_icon_for_file(&cur_exe).unwrap(),
            "for current executable",
        );
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
            ) -> Result<Icon, IconError> {
                let path = path.as_ref();
                assert!(path.as_os_str() == path.canonicalize().unwrap().as_os_str());
                Ok(self.get_default_icon())
            }

            fn get_icon_for_url(&self, _url: &str) -> Result<Icon, IconError> {
                panic!("expected file, not url")
            }
        }

        use tempfile;
        let curdir = std::env::current_dir().unwrap();
        let dir = tempfile::tempdir_in(&curdir).unwrap();
        let dirname = dir.path().strip_prefix(curdir).unwrap();

        let filename = dirname.join("test.txt");

        let _file = std::fs::File::create(&filename).unwrap();

        let icon_type = super::IconType(super::IconVariant::File(filename));
        let mock = MockIcon;
        mock.load_icon(icon_type);
    }

    #[test]
    fn common_urls_are_iconlike() {
        // test urls that default macos has support for
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let happycase_urls = vec![
            "http://example.com",
            "https://example.com",
            "mailto:example@example.com",
        ];

        #[cfg(all(unix, not(target_os = "macos")))]
        let happycase_urls: Vec<&str> = Vec::new();

        #[cfg(windows)]
        let happycase_urls: Vec<_> = happycase_urls
            .into_iter()
            .chain(
                [
                    "accountpicturefile:",
                    "AudioCD:",
                    "batfile:",
                    "fonfile:",
                    "hlpfile:",
                    "regedit:",
                    "read:",
                ]
                .into_iter(),
            )
            .collect();

        let sadcase_urls = vec![
            "totallynonexistantprotocol:",
            "http:", // malformed url
        ];

        #[cfg(windows)]
        let sadcase_urls: Vec<_> = sadcase_urls
            .into_iter()
            .chain(
                [
                    "anifile:", // uses %1 as the icon
                    "tel:",     // defined but empty on windows
                ]
                .into_iter(),
            )
            .collect();

        let os = IconSettings::default();

        for url in happycase_urls.iter() {
            let icon = os
                .get_icon_for_url(url)
                .expect(&format!(r#"could not load icon for url "{}""#, url));

            iconlike(icon, &format!("for common url {}", url));
        }

        for url in sadcase_urls.iter() {
            os.get_icon_for_url(url)
                .expect_err(&format!(r#"was able to load icon for url "{}""#, url));
        }
    }

    #[test]
    fn common_files_are_iconlike() {
        let dir = tempfile::tempdir().unwrap();
        let files = ["foo.txt", "bar.html", "baz.png", "bat.pdf"];

        let os = IconSettings::default();
        for f in files {
            let path = dir.path().join(f);
            let file = std::fs::File::create(&path).unwrap();
            file.sync_all().unwrap();

            assert!(path.exists());
            os.get_icon_for_file(&path)
                .expect(&format!("No Icon for file: {f}"));
        }
    }

    #[test]
    fn load_custom_icons() {
        use super::*;
        // example test pbm image
        let pbm_bytes = "P1\n2 2\n1 0 1 0".as_bytes();
        // example test svg
        let test_svg = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("icon/jolly.svg");

        let dir = tempfile::tempdir().unwrap();

        let pbm_fn = dir.path().join("test.pbm");

        std::fs::write(&pbm_fn, pbm_bytes).unwrap();

        let os = IconSettings::default();

        let pbm_icon = os.inner_load_icon(IconType::custom(pbm_fn)).unwrap();
        assert!(matches!(pbm_icon.data(), iced_native::image::Data::Path(_)));

        os.inner_load_icon(IconType::custom("file_with_no_extension"))
            .unwrap_err();

        os.inner_load_icon(IconType::custom("unsupported_icon_type.pdf"))
            .unwrap_err();

        let svg_icon = os.inner_load_icon(IconType::custom(test_svg)).unwrap();
        assert!(matches!(
            svg_icon.data(),
            iced_native::image::Data::Rgba {
                width: w,
                height: h,
                pixels: _
            }
            if *w == DEFAULT_ICON_SIZE as u32 && *h == DEFAULT_ICON_SIZE as u32
        ));
    }
}
