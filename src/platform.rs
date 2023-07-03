use crate::ui;
use opener;
use std::error;
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    OpenerError(opener::OpenError),
    IoError(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Error::OpenerError(err) => {
                if let opener::OpenError::ExitStatus { stderr: e, .. } = err {
                    f.write_str(e)
                } else {
                    err.fmt(f)
                }
            }
            Error::IoError(err) => err.fmt(f),
        }
    }
}

impl error::Error for Error {}

const DEFAULT_ACCENT_COLOR: ui::Color = ui::Color(csscolorparser::Color {
    r: 0x5E as f64 / 255.0,
    g: 0x7C as f64 / 255.0,
    b: 0xE2 as f64 / 255.0,
    a: 1.0,
});

// based on subprocess crate
#[cfg(unix)]
pub(crate) mod os {
    use crate::ui;
    use std::ffi::OsStr;
    use std::process::Command;

    pub const SHELL: [&str; 2] = ["sh", "-c"];
    pub const ACCENT_COLOR: &'static ui::Color = &super::DEFAULT_ACCENT_COLOR;

    // run a subshell and interpret results
    pub fn system(cmdstr: impl AsRef<OsStr>) -> std::io::Result<std::process::Child> {
        Command::new(SHELL[0]).args(&SHELL[1..]).arg(cmdstr).spawn()
    }
}

#[cfg(windows)]
pub(crate) mod os {
    use crate::ui;
    use std::ffi::OsStr;
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use windows::UI::ViewManagement::{UIColorType, UISettings};

    pub const SHELL: [&str; 2] = ["cmd.exe", "/c"];

    // try and get the windows accent color. This wont work for
    // windows < 10
    fn try_get_color() -> Option<ui::Color> {
        let settings = UISettings::new().ok()?;
        let color = settings.GetColorValue(UIColorType::Accent).ok()?;
        Some(ui::Color(csscolorparser::Color::from_rgba8(
            color.R, color.G, color.B, 255,
        )))
    }

    lazy_static::lazy_static! {
    pub static ref ACCENT_COLOR : ui::Color = {
        // if we cannot get the windows accent color, default to the unix one
        if let Some(color) = try_get_color() {
        color
        } else {
        super::DEFAULT_ACCENT_COLOR
        }
    };
    }

    // run a subshell and interpret results
    pub fn system(cmdstr: impl AsRef<OsStr>) -> std::io::Result<std::process::Child> {
        Command::new(SHELL[0])
            //spawn the command window without a console (CREATE_NO_WINDOW)
            // see https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags
            .creation_flags(0x08000000)
            .args(&SHELL[1..])
            .arg(cmdstr)
            .spawn()
    }
}

pub fn system(cmdstr: impl AsRef<OsStr>) -> Result<(), Error> {
    os::system(cmdstr).map(|_| ()).map_err(Error::IoError)
}

pub fn accent_color() -> ui::Color {
    os::ACCENT_COLOR.clone()
}

pub fn open_file<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    opener::open(path.as_ref().as_os_str()).map_err(Error::OpenerError)
}
