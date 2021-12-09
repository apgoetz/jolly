use std::path::Path;
use std::process::{Command,Stdio};
pub fn open_file<P:AsRef<Path>>(path : P) -> Result<(), super::Error> {
    let result = Command::new("open")
	.arg(path.as_ref().as_os_str())
	.stdin(Stdio::null())
	.stdout(Stdio::null())
	.stderr(Stdio::null())
	.status().map_err(super::Error::IOError)?;
    if result.success() {
	Ok(())
    } else {
	Err(super::Error::OpenError(path.as_ref().display().to_string()))
    }
}

