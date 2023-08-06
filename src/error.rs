// jolly error types

use super::entry;
use super::platform;
use iced;
use std::error;
use std::fmt;
use std::io;
#[derive(Debug)]
pub enum Error {
    StoreError(entry::Error),
    IcedError(iced::Error),
    IoError(Option<String>, io::Error),
    ParseError(String),
    ContextParseError(String, String),
    PlatformError(platform::Error),
    CustomError(String),
    FinalMessage(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::StoreError(e) => {
                write!(f, "while parsing jolly.toml: \n")?;
                e.fmt(f)
            }
            Error::IcedError(e) => e.fmt(f),
            Error::IoError(file, e) => {
                if let Some(file) = file {
                    write!(f, "with file '{file}': \n")?;
                } else {
                    f.write_str("IO Error:\n")?;
                }
                e.fmt(f)
            }
            Error::ParseError(e) => f.write_str(e),
            Error::ContextParseError(file, e) => {
                write!(f, "while parsing '{file}':\n")?;
                e.fmt(f)
            }
            Error::PlatformError(e) => e.fmt(f),

            Error::CustomError(s) => f.write_str(s),
            // not really an error, used to represent final message in UI
            Error::FinalMessage(s) => f.write_str(s),
        }
    }
}

impl error::Error for Error {}
