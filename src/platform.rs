use opener;
use std::error;
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::Path;
use std::process::Command;

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

// based on subprocess crate
#[cfg(unix)]
mod os {
    pub const SHELL: [&str; 2] = ["sh", "-c"];
}

#[cfg(windows)]
mod os {
    pub const SHELL: [&str; 2] = ["cmd.exe", "/c"];
}
use os::*;

// run a subshell and interpret results
pub fn system(cmdstr: impl AsRef<OsStr>) -> Result<(), Error> {
    Command::new(SHELL[0])
        .args(&SHELL[1..])
        .arg(cmdstr)
        .spawn()
        .map(|_| ())
        .map_err(Error::IoError)
}

pub fn open_file<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    opener::open(path.as_ref().as_os_str()).map_err(Error::OpenerError)
}
