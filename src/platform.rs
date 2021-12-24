use opener;
use std::path::Path;

pub type Error = opener::OpenError;

pub fn open_file<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    opener::open(path.as_ref().as_os_str())
}
