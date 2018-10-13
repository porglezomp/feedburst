use std::ffi::OsStr;
use std::process::Command;

use crate::error::Error;

pub fn open_url<T: AsRef<OsStr>>(url: T) -> Result<(), Error> {
    let exit_status = Command::new("open").arg(&url).spawn()?.wait()?;
    if exit_status.success() {
        Ok(())
    } else {
        let msg = format!("Failed opening url {}", url.as_ref().to_string_lossy());
        Err(Error::Msg(msg))
    }
}
