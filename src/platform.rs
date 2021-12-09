use std::fmt;
use std::path::Path;
use std::process::{Command,Stdio};

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    OpenError(String)
}

impl fmt::Display for Error {
    fn fmt(&self, f:&mut fmt::Formatter<'_>) -> fmt::Result {
	match self {
	    Error::IOError(e) => e.fmt(f),
	    Error::OpenError(p) => {
		write!(f, "Could not open path: ")?;
		p.fmt(f)
	    }
	}
    }
}

#[cfg(target_os = "macos")]
const OPEN_CMD : &str = "open";

#[cfg(target_os = "windows")]
const OPEN_CMD : &str = "start";

#[cfg(target_os = "linux")]
const OPEN_CMD : &str = "xdg-open";


pub fn open_file<P:AsRef<Path>>(path : P) -> Result<(), Error> {
    let result = Command::new(OPEN_CMD)
	.arg(path.as_ref().as_os_str())
	.stdin(Stdio::null())
	.stdout(Stdio::null())
	.stderr(Stdio::null())
	.status().map_err(Error::IOError)?;
    if result.success() {
	Ok(())
    } else {
	Err(Error::OpenError(path.as_ref().display().to_string()))
    }
}



