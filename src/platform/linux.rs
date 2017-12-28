use std::ffi::OsStr;
use std::process::Command;

use error::Error;

pub fn open_url<T: AsRef<OsStr>>(url: T) -> Result<(), Error> {
    let mut last_err = Err(Error::Msg("Unknown error".into()));
    for program in &["xdg-open", "gnome-open", "kde-open"] {
        match Command::new(program).arg(&url).spawn() {
            Ok(mut child) => {
                let exit_status = child.wait()?;
                if exit_status.success() {
                    return Ok(());
                } else {
                    let msg = format!("Failed opening url {}", url.as_ref().to_string_lossy());
                    return Err(Error::Msg(msg));
                }
            }
            Err(err) => {
                let msg = format!("Unable to open {}: {:?}", program, err);
                last_err = Err(Error::Msg(msg));
            }
        }
    }
    last_err
}
