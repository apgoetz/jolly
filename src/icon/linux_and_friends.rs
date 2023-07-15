#![cfg(all(unix, not(target_os = "macos")))]
// for now, this covers linux and the bsds
use super::{icon_from_svg, Context, Icon, IconError, DEFAULT_ICON_SIZE};

use std::io::Read;

use serde;
use xdg_mime::SharedMimeInfo;

// set in build script
pub const DEFAULT_THEME: &str = env!("JOLLY_DEFAULT_THEME");

// TODO make mime sniff size a config parameter?
const SNIFFSIZE: usize = 8 * 1024;

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Os {
    pub theme: String,
    xdg_folder: Option<String>,
}

impl Default for Os {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.into(),
            xdg_folder: None,
        }
    }
}

impl super::IconInterface for Os {
    fn get_default_icon(&self) -> Result<Icon, IconError> {
        self.get_icon_for_iname("text-x-generic")
    }

    fn get_icon_for_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<Icon, IconError> {
        let path = path.as_ref();
        let inames = self.get_iname_for_file(path)?;

        for iname in &inames {
            let icon = self.get_icon_for_iname(iname);
            if icon.is_ok() {
                return icon;
            }
        }
        Err(format!("No valid icon. inames were {:?}", inames).into())
    }

    fn get_icon_for_url(&self, url: &str) -> Result<Icon, IconError> {
        let iname = self.get_iname_for_url(url)?;
        self.get_icon_for_iname(&iname)
    }

    // for linux apps we need to make sure there are some default mime
    // types specified since CI is run headless
}

impl Os {
    fn get_iname_for_url(&self, p: &str) -> Result<String, IconError> {
        use url::Url;

        let url = Url::parse(p).context("Url is not valid: {p}")?;

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

        let output = cmd.output().context("xdg-settings unsuccessful")?;

        // assume that we got back utf8 for the application name
        let mut handler =
            String::from_utf8(output.stdout).context("invalid utf8 from xdg-settings")?;
        if handler.ends_with("\n") {
            handler.pop();
        }

        if handler.is_empty() {
            Err("no scheme handler found".into())
        } else {
            Ok(handler)
        }
    }

    fn get_iname_for_file<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<Vec<String>, IconError> {
        let filename = path
            .as_ref()
            .as_os_str()
            .to_str()
            .context("filename not valid unicode")?;

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

        let data: Option<Vec<_>>;

        // TODO, handle files we can see but not read
        if let Ok(mut file) = std::fs::File::open(filename) {
            let mut buf = vec![0u8; SNIFFSIZE];
            if let Ok(numread) = file.read(buf.as_mut_slice()) {
                buf.truncate(numread);
                data = Some(buf);
            } else {
                data = None;
            }
        } else {
            data = None;
        }

        // this next part is a little gross, but xdg_mime currently
        // hardcodes a mimetype of application/x-zerosize if a file is
        // empty. So we need to run the mime sniffing 2 different
        // ways, once with data and once without, and then lump them
        // all together to find whichever one creates an icon

        let guess = match data {
            Some(buf) => mimeinfo.guess_mime_type().path(filename).data(&buf).guess(),
            None => mimeinfo.guess_mime_type().path(filename).guess(),
        };

        let fn_guess = mimeinfo.get_mime_types_from_file_name(filename);

        let allmimes = std::iter::once(guess.mime_type().clone()).chain(fn_guess.into_iter());

        let allparents = allmimes
            .clone()
            .flat_map(|m| mimeinfo.get_parents(&m).unwrap_or_default().into_iter());

        Ok(allmimes
            .chain(allparents)
            .flat_map(|m| mimeinfo.lookup_icon_names(&m).into_iter())
            .collect())
    }

    fn get_icon_for_iname(&self, icon_name: &str) -> Result<Icon, IconError> {
        use freedesktop_icons::lookup;

        let icon_name = icon_name.strip_suffix(".desktop").unwrap_or(icon_name);

        let icon_path = lookup(icon_name)
            .with_size(DEFAULT_ICON_SIZE)
            .with_theme(&self.theme)
            .find()
            .ok_or("Could not lookup icon")?;

        // TODO handle other supported icon types
        if icon_path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("png"))
        {
            Ok(iced::widget::image::Handle::from_path(icon_path))
        } else if icon_path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
        {
            icon_from_svg(&icon_path)
        } else {
            Err(format!(
                "unsupported icon file type for icon {}",
                icon_path.to_string_lossy()
            )
            .into())
        }
    }
}

#[cfg(test)]
mod tests {

    use std::fs::{create_dir, write};
    use std::process::Command;
    use tempfile;

    use super::*;

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

        assert!(os.get_iname_for_url("http://google.com").is_err());
        assert!(os.get_iname_for_url("tel:12345").is_ok());
    }

    #[test]
    fn test_load_file() {
        let dir = tempfile::tempdir().unwrap();
        // build a mock xdg with the ability to handle rust source and nothing else
        let xdg = MockXdg::new();
        xdg.register_mime("text/x-rust", "rs");
        let os = xdg.os(DEFAULT_THEME);
        let file = dir.path().join("test.rs");
        std::fs::File::create(&file).unwrap();
        let mimetypes = os.get_iname_for_file(file).unwrap();
        assert!(
            mimetypes.contains(&"text-x-rust".into()),
            "actual {:?}",
            mimetypes
        );
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
