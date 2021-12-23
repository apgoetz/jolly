use std::fmt;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    OpenError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IOError(e) => e.fmt(f),
            Error::OpenError(p) => {
                write!(f, "Could not open path: ")?;
                p.fmt(f)
            }
        }
    }
}

struct PlatformOpts {
    open_cmd: &'static str,
    ignore_errors: bool,
}

#[cfg(target_os = "macos")]
const OPTS: PlatformOpts = PlatformOpts {
    open_cmd: "open",
    ignore_errors: false,
};

#[cfg(target_os = "windows")]
const OPTS: PlatformOpts = PlatformOpts {
    open_cmd: "explorer.exe",
    ignore_errors: true,
};

#[cfg(target_os = "linux")]
const OPTS: PlatformOpts = PlatformOpts {
    open_cmd: "xdg-open",
    ignore_errors: false,
};

pub fn open_file<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    let result = Command::new(OPTS.open_cmd)
        .arg(path.as_ref().as_os_str())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(Error::IOError)?;
    if result.success() || OPTS.ignore_errors {
        Ok(())
    } else {
        Err(Error::OpenError(path.as_ref().display().to_string()))
    }
}
