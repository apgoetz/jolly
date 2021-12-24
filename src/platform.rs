use opener;
use std::path::Path;

pub type Error = opener::OpenError;

pub fn open_file<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    println!("{}", path.as_ref().display());
    opener::open(path.as_ref().as_os_str())
}
