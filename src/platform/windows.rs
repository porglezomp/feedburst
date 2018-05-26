use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use error::Error;

fn app_data_dir() -> Result<PathBuf, Error> {
    if let Some(app_data_dir) = env::var_os("APPDATA") {
        Ok(Path::new(&app_data_dir).join("Feedburst"))
    } else {
        Err(Error::Msg("Unable to find the APPDATA directory".into()))
    }
}

pub fn data_path(path: &str) -> Result<PathBuf, Error> {
    let path = app_data_dir()?.join(path).parent().unwrap().into();
    fs::create_dir_all(&path).map_err(|err| {
        Error::Msg(format!(
            "Error creating feeds directory {:?}: {}",
            path, err
        ))
    })?;
    Ok(path)
}

pub fn config_path() -> Result<PathBuf, Error> {
    let path = app_data_dir()?;
    fs::create_dir_all(&path).map_err(|err| {
        Error::Msg(format!(
            "Error creating config directory {:?}: {}",
            path, err
        ))
    })?;
    Ok(path.join("config.feeds"))
}

pub fn open_url<T: AsRef<OsStr>>(url: T) -> Result<(), Error> {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C").arg("start");
    if let Some(s) = url.as_ref().to_str() {
        cmd.arg(s.replace("&", "^&"));
    } else {
        cmd.arg(url.as_ref());
    }
    let exit_status = cmd.spawn()?.wait()?;
    if exit_status.success() {
        Ok(())
    } else {
        let msg = format!("Failed opening url {}", url.as_ref().to_string_lossy());
        Err(Error::Msg(msg))
    }
}
