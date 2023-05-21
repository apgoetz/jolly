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
        // Final message is used when we want to say something, but it
        // isn't an error, so we don't say error
        if !matches!(self, Error::FinalMessage(_)) {
            write!(f, "Error: ")?;
        }

        match self {
            Error::StoreError(e) => e.fmt(f),
            Error::IcedError(e) => e.fmt(f),
            Error::IoError(e) => e.fmt(f),
            Error::ParseError(e) => e.fmt(f),
            Error::PlatformError(e) => e.fmt(f),
            Error::CustomError(s) => f.write_str(s),
            Error::FinalMessage(s) => f.write_str(s),
        }
    }
}

impl error::Error for Error {}
