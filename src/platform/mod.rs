use std::fmt;

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


mod macos;
pub use macos::*;

