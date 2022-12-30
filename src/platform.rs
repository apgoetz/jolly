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

const DEFAULT_ACCENT_COLOR: iced_native::Color = iced_native::Color {
    r: 92.0 / 255.0,
    g: 144.0 / 255.0,
    b: 226.0 / 255.0,
    a: 1.0,
};

// based on subprocess crate
#[cfg(unix)]
mod os {
    use std::ffi::OsStr;
    use std::process::Command;

    const SHELL: [&str; 2] = ["sh", "-c"];
    pub const ACCENT_COLOR: &'static iced_native::Color = &super::DEFAULT_ACCENT_COLOR;

    // run a subshell and interpret results
    pub fn system(cmdstr: impl AsRef<OsStr>) -> std::io::Result<std::process::Child> {
        Command::new(SHELL[0]).args(&SHELL[1..]).arg(cmdstr).spawn()
    }
}

#[cfg(windows)]
mod os {
    use std::ffi::OsStr;
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use windows::UI::ViewManagement::{UIColorType, UISettings};

    const SHELL: [&str; 2] = ["cmd.exe", "/c"];

    // try and get the windows accent color. This wont work for
    // windows < 10
    fn try_get_color() -> Option<iced_native::Color> {
        let settings = UISettings::new().ok()?;
        let color = settings.GetColorValue(UIColorType::Accent).ok()?;
        Some(iced_native::Color::from_rgb8(color.R, color.G, color.B))
    }

    lazy_static::lazy_static! {
    pub static ref ACCENT_COLOR : iced_native::Color = {
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

pub fn accent_color() -> iced_native::Color {
    *os::ACCENT_COLOR
}

pub fn open_file<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    opener::open(path.as_ref().as_os_str()).map_err(Error::OpenerError)
}
