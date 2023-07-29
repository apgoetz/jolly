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
    IoError(io::Error),
    ParseError(String),
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
            Error::IoError(e) => {
                write!(f, "could not access jolly.toml: \n")?;
                e.fmt(f)
            }
            Error::ParseError(e) => {
                write!(f, "while parsing jolly.toml: \n")?;
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
