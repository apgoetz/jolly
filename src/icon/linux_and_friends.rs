#![cfg(all(unix, not(target_os = "macos")))]
// for now, this covers linux and the bsds
use super::{Icon, FALLBACK_ICON};

use lazy_static::lazy_static;
use serde;
use xdg_mime::SharedMimeInfo;

// set in build script
pub const DEFAULT_THEME: &str = env!("JOLLY_DEFAULT_THEME");

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Os {
    pub theme: String,
    pub icon_size: u16,
    xdg_folder: Option<String>,
}

impl Default for Os {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.into(),
            icon_size: 48,
            xdg_folder: None,
        }
    }
}

impl super::IconInterface for Os {
    fn get_default_icon(&self) -> Icon {
        if let Some(icon) = self.get_icon_for_iname("text-x-generic") {
            icon
        } else {
            // if you can't load the generic icon, you get a grey box
            FALLBACK_ICON.clone()
        }
    }

    fn get_icon_for_file<P: AsRef<std::path::Path>>(&self, path: P) -> Option<Icon> {
        let path = path.as_ref();
        let inames = self.get_iname_for_file(path);

        for iname in inames {
            let icon = self.get_icon_for_iname(&iname);
            if icon.is_some() {
                return icon;
            }
        }
        None
    }

    fn get_icon_for_url(&self, url: &str) -> Option<Icon> {
        let iname = self.get_iname_for_url(url)?;
        self.get_icon_for_iname(&iname)
    }

    // for linux apps we need to make sure there are some default mime
    // types specified since CI is run headless
}

impl Os {
    fn get_iname_for_url(&self, p: &str) -> Option<String> {
        use url::Url;

        let url = Url::parse(p).ok()?;

        use std::process::Command;
        let mut cmd = Command::new("xdg-settings");

        cmd.arg("get")
            .arg("default-url-scheme-handler")
            .arg(url.scheme());

        // if xdg folder is specified, we use this to override the
        // location we look for url settings
        if let Some(f) = &self.xdg_folder {
            cmd.env("XDG_DATA_HOME", f);
            cmd.env("XDG_DATA_DIRS", f);
            cmd.env("HOME", f);
            cmd.env("DE", "generic");
        }

        let output = cmd.output().ok()?;

        // assume that we got back utf8 for the applcation name
        chomp(String::from_utf8(output.stdout).ok())
    }

    fn get_iname_for_file<P: AsRef<std::path::Path>>(&self, path: P) -> Vec<String> {
        let filename = match path.as_ref().as_os_str().to_str() {
            Some(s) => s,
            None => return vec![],
        };

        use once_cell::sync::OnceCell;

        static MIMEINFO: OnceCell<SharedMimeInfo> = OnceCell::new();

        // if xdg folder is specified, we use this to override the
        // location we look for mimetype settings
        let mimeinfo = match &self.xdg_folder {
            Some(f) => {
                let m = Box::new(SharedMimeInfo::new_for_directory(f));
                Box::leak(m) // ok because only used in testing
            }
            None => MIMEINFO.get_or_init(SharedMimeInfo::new),
        };

        let mimes = mimeinfo.get_mime_types_from_file_name(filename);

        let mut icon_names = Vec::with_capacity(mimes.len());

        for m in mimes {
            icon_names.append(&mut mimeinfo.lookup_icon_names(&m));
        }

        if icon_names[0] == "application-octet-stream" {
            if let Some(mut name) = self.sniff_mimetypes(path, &mimeinfo) {
                name.append(&mut icon_names);
                return name;
            }
        }
        icon_names
    }

    fn sniff_mimetypes<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        mimeinfo: &SharedMimeInfo,
    ) -> Option<Vec<String>> {
        use std::io::Read;

        const SNIFFSIZE: usize = 8 * 1024;
        let mut file = std::fs::File::open(path).ok()?;
        let mut buf = vec![0u8; SNIFFSIZE];
        let numread = file.read(buf.as_mut_slice()).ok()?;
        buf.truncate(numread);

        let (mime, _score) = mimeinfo.get_mime_type_for_data(&buf)?;

        Some(mimeinfo.lookup_icon_names(&mime))
    }

    // for now do it the lame way and do our own svg rendering
    fn icon_from_svg(&self, path: &std::path::Path) -> Option<Icon> {
        use resvg::usvg::TreeParsing;
        let svg_data = std::fs::read(path).ok()?;
        let utree = resvg::usvg::Tree::from_data(&svg_data, &Default::default()).ok()?;

        let icon_size = self.icon_size as u32;

        let mut pixmap = resvg::tiny_skia::Pixmap::new(icon_size, icon_size)?;

        let rtree = resvg::Tree::from_usvg(&utree);

        // we have non-square svg
        if rtree.size.width() != rtree.size.height() {
            return None;
        }

        let scalefactor = icon_size as f32 / rtree.size.width();
        let transform = resvg::tiny_skia::Transform::from_scale(scalefactor, scalefactor);

        rtree.render(transform, &mut pixmap.as_mut());

        Some(Icon::from_pixels(
            icon_size,
            icon_size,
            pixmap.take().leak(),
        ))
    }

    fn get_icon_for_iname(&self, icon_name: &str) -> Option<Icon> {
        use freedesktop_icons::lookup;

        let icon_name = icon_name.strip_suffix(".desktop").unwrap_or(icon_name);

        let icon_path = lookup(icon_name)
            .with_size(self.icon_size)
            .with_theme(&self.theme)
            .find()?;

        if icon_path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("png"))
        {
            Some(iced::widget::image::Handle::from_path(icon_path))
        } else if icon_path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
        {
            self.icon_from_svg(&icon_path)
        } else {
            None
        }
    }
}

fn chomp(s: Option<String>) -> Option<String> {
    s.map(|mut s| {
        if s.ends_with("\n") {
            s.pop();
        }
        s
    })
    .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {

    use std::fs::{create_dir, write};
    use std::process::Command;
    use tempfile;

    use super::*;
    use crate::icon::IconInterface;

    // helper struct to allow building mock xdg data for testing
    struct MockXdg(tempfile::TempDir);

    impl MockXdg {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();

            let p = dir.path();
            create_dir(p.join("applications")).unwrap();
            create_dir(p.join("mime")).unwrap();
            Self(dir)
        }

        fn add_app(&self, appname: &str) {
            let p = self.0.path().join(format!("applications/{}", appname));
            write(p, b"[Desktop Entry]\nExec=/bin/sh").unwrap();
        }

        fn register_url(&self, url: &str, appname: &str) {
            let out = Command::new("xdg-settings")
                .args(["set", "default-url-scheme-handler", url, appname])
                .env("XDG_DATA_HOME", self.0.path())
                .env("XDG_DATA_DIRS", self.0.path())
                .env("HOME", self.0.path())
                .env("DE", "generic")
                .env("XDG_UTILS_DEBUG_LEVEL", "2")
                .output()
                .unwrap();
            println!(
                "registering_url: {} -> {}  status: {} {}",
                url,
                appname,
                out.status,
                String::from_utf8(out.stderr).unwrap()
            );
        }

        fn register_mime(&self, mimetype: &str, extension: &str) {
            let filename = mimetype.split("/").last().unwrap();
            let filename = self.0.path().join(format!("{}.xml", filename));
            let text = format!(
                r#"<?xml version="1.0" encoding="utf-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
<mime-type type="{}">
  <glob pattern="*.{}"/>
</mime-type>
</mime-info>
"#,
                mimetype, extension
            );

            write(&filename, text.as_bytes()).unwrap();
            let out = Command::new("xdg-mime")
                .args([
                    "install",
                    "--novendor",
                    "--mode",
                    "user",
                    filename.to_str().unwrap(),
                ])
                .env("XDG_DATA_HOME", self.0.path())
                .env("XDG_DATA_DIRS", self.0.path())
                .env("HOME", self.0.path())
                .env("DE", "generic")
                .env("XDG_UTILS_DEBUG_LEVEL", "2")
                .output()
                .unwrap();
            println!(
                "registering_mime: {} status: {} {}",
                mimetype,
                out.status,
                String::from_utf8(out.stderr).unwrap()
            );
        }

        fn os(&self, theme: &str) -> Os {
            Os {
                theme: theme.into(),
                icon_size: 48,
                xdg_folder: Some(self.0.path().to_str().unwrap().into()),
            }
        }
    }

    #[test]
    fn test_load_icon() {
        // build a mock xdg with the ability to handle telephone and nothing else
        let xdg = MockXdg::new();
        xdg.add_app("test.desktop");
        xdg.register_url("tel", "test.desktop");
        xdg.register_mime("text/x-rust", "rs");
        let os = xdg.os(DEFAULT_THEME);

        assert!(os.get_iname_for_url("http://google.com").is_none());
        assert!(os.get_iname_for_url("tel:12345").is_some());
    }

    #[test]
    fn test_load_file() {
        // build a mock xdg with the ability to handle rust source and nothing else
        let xdg = MockXdg::new();
        xdg.register_mime("text/x-rust", "rs");
        let os = xdg.os(DEFAULT_THEME);
        let mimetypes = os.get_iname_for_file("test.rs");
        assert!(mimetypes.contains(&"text-x-rust".into()));

        assert!(
            mimetypes[0] != "application-octet-stream",
            "mimetypes: {:?}",
            mimetypes
        );
    }

    #[test]
    fn test_default_icon_always_works() {
        let xdg = MockXdg::new();
        let icon = xdg.os("nonexistant_theme").get_default_icon();

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let hash_eq_fallback = |icon: &Icon| {
            let mut ihash = DefaultHasher::new();
            let mut fhash = DefaultHasher::new();
            icon.hash(&mut ihash);
            FALLBACK_ICON.hash(&mut fhash);
            ihash.finish() == fhash.finish()
        };

        // cheat and use hash to see if we have gotten the fallback icon
        assert!(hash_eq_fallback(&icon));

        // with a default icon loaded, we now load a path for the icon
        let icon = xdg.os(DEFAULT_THEME).get_default_icon();
        assert!(!hash_eq_fallback(&icon));
    }

    #[test]
    fn can_load_svg_icons() {
        // freedesktop_icons falls back to using the icon name as a
        // file path if it cant find it otherwise. so we can use this
        // to force loading the jolly svg icon
        let svg_icon = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("icon/jolly");

        let icon = Os::default()
            .get_icon_for_iname(svg_icon.as_os_str().to_str().unwrap())
            .unwrap();
        // expect pixel data from the icon
        assert!(matches!(
            icon.data(),
            iced_native::image::Data::Rgba {
                width: _,
                height: _,
                pixels: _
            }
        ));
    }
}
