use std::env;
use std::path::PathBuf;

use error::Error;

pub fn data_path(path: &str) -> Result<PathBuf, Error> {
    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        Ok(path.into())
    } else {
        let xdg = ::xdg::BaseDirectories::with_prefix(::APP_NAME)
            .map_err(|err| Error::Msg(format!("{}", err)))?;
        if let Some(path) = xdg.find_data_file(path) {
            Ok(path)
        } else {
            xdg.place_data_file(path)
                .map_err(|err| Error::Msg(format!("{}", err)))
        }
    }
}

pub fn config_path() -> Result<PathBuf, Error> {
    if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
        Ok(path.into())
    } else {
        let xdg = ::xdg::BaseDirectories::with_prefix(::APP_NAME)
            .map_err(|err| Error::Msg(format!("{}", err)))?;
        if let Some(path) = xdg.find_config_file("config.feeds") {
            Ok(path)
        } else {
            xdg.place_config_file("config.feeds")
                .map_err(|err| Error::Msg(format!("{}", err)))
        }
    }
}
