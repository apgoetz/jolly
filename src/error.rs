// jolly error types

use super::store;
use super::platform;
use std::fmt;
use std::error;
use iced;

#[derive(Debug)]
pub enum Error {
    StoreError(store::Error),
    IcedError(iced::Error),
    PlatformError(platform::Error),
    CustomError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f:&mut fmt::Formatter<'_>) -> fmt::Result {
	write!(f, "Error: ")?;
	match self {
	    Error::StoreError(e) => e.fmt(f),
	    Error::IcedError(e) => e.fmt(f),
	    Error::PlatformError(e) => e.fmt(f),
	    Error::CustomError(s) => f.write_str(s)
	}
    }
}

impl error::Error for Error {}