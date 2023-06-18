#![cfg(all(unix, not(target_os = "macos")))]
// for now, this covers linux and the bsds
use super::Icon;

use serde;
use xdg_mime::SharedMimeInfo;

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Os {
    pub theme: String,
    xdg_folder: Option<String>,
}

impl Default for Os {
    fn default() -> Self {
        Self {
            theme: "gnome".into(),
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
            Icon::from_pixels(1, 1, &[127, 127, 127, 255])
        }
    }

    fn get_icon_for_file<P: AsRef<std::path::Path>>(&self, path: P) -> Option<Icon> {
        for iname in self.get_iname_for_file(path) {
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
        icon_names
    }

    fn get_icon_for_iname(&self, icon_name: &str) -> Option<Icon> {
        use freedesktop_icons::lookup;

        let icon_name = icon_name.strip_suffix(".desktop").unwrap_or(icon_name);

        let icon_path = lookup(icon_name)
            .with_size(48) // spec says 48 is default
            .with_theme(&self.theme)
            .with_cache()
            .find();

        icon_path.map(iced::widget::image::Handle::from_path)
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

        fn add_icon(&self, themename: &str, iconpath: &str) {
            let p = self
                .0
                .path()
                .join(format!("icons/{}/48x48/{}", themename, iconpath));
            //write empty file for now
            write(p, b"").unwrap();
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
        let os = xdg.os("gnome");

        assert!(os.get_iname_for_url("http://google.com").is_none());
        assert!(os.get_iname_for_url("tel:12345").is_some());
    }

    #[test]
    fn test_load_file() {
        // build a mock xdg with the ability to handle rust source and nothing else
        let xdg = MockXdg::new();
        xdg.register_mime("text/x-rust", "rs");
        let os = xdg.os("gnome");
        assert!(os
            .get_iname_for_file("test.rs")
            .contains(&"text-x-rust".into()));
    }

    #[test]
    fn test_default_icon_always_works() {
        let xdg = MockXdg::new();
        let icon = xdg.os("gnome").get_default_icon();

        // with no icons added we default to internal picture
        assert!(matches!(
            icon.data(),
            iced_native::image::Data::Rgba {
                width: _,
                height: _,
                pixels: _
            }
        ));

        // with a default icon loaded, we now load a path for the icon
        xdg.add_icon("gnome", "mimetypes/text-x-generic.png");
        let icon = xdg.os("gnome").get_default_icon();
        assert!(matches!(icon.data(), iced_native::image::Data::Path(_)));
    }
}
