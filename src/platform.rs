use opener;
use std::error;
use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub struct Error(opener::OpenError);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            opener::OpenError::ExitStatus { stderr: e, .. } => f.write_str(e),
            _ => self.0.fmt(f),
        }
    }
}

impl error::Error for Error {}

pub fn open_file<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    opener::open(path.as_ref().as_os_str()).map_err(Error)
}
