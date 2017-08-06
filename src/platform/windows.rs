use std::path::{Path, PathBuf};
use std::{fs, env};

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
            path,
            err
        ))
    })?;
    Ok(path)
}

pub fn config_path() -> Result<PathBuf, Error> {
    let path = app_data_dir()?;
    fs::create_dir_all(&path).map_err(|err| {
        Error::Msg(format!(
            "Error creating config directory {:?}: {}",
            path,
            err
        ))
    })?;
    Ok(path.join("config.feeds"))
}
