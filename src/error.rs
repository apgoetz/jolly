// jolly error types

use super::platform;
use super::store;
use iced;
use std::error;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    StoreError(store::Error),
    IcedError(iced::Error),
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
            Error::PlatformError(e) => e.fmt(f),
            Error::CustomError(s) => f.write_str(s),
            Error::FinalMessage(s) => f.write_str(s),
        }
    }
}

impl error::Error for Error {}
