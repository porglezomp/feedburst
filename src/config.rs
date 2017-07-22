use std::path::PathBuf;
#[cfg(windows)]
use std::path::Path;

use error::Error;


#[cfg(unix)]
pub fn get_feed_path(name: &str) -> Result<PathBuf, Error> {
    let path = format!("feeds/{}.feed", name);
    Ok(::xdg::BaseDirectories::with_prefix(::APP_NAME)
        .map_err(|e| Error::Msg(format!("{}", e)))?
        .place_data_file(&path)
        .map_err(|e| Error::Msg(format!("{}", e)))?)
}

#[cfg(unix)]
pub fn get_config_path() -> Result<PathBuf, Error> {
    Ok(::xdg::BaseDirectories::with_prefix(::APP_NAME)
        .map_err(|e| Error::Msg(format!("{}", e)))?
        .place_config_file("config.feeds")
        .map_err(|e| Error::Msg(format!("{}", e)))?)
}

#[cfg(windows)]
fn app_data_dir() -> Result<PathBuf, Error> {
    if let Some(app_data_dir) = ::std::env::var_os("APPDATA") {
        Ok(Path::new(&app_data_dir).join("Feedburst"))
    } else {
        Err(Error::Msg("Unable to find the APPDATA directory".into()))
    }
}

#[cfg(windows)]
pub fn get_feed_path(name: &str) -> Result<PathBuf, Error> {
    let path = app_data_dir()?.join("feeds");
    ::std::fs::create_dir_all(&path).map_err(|_| {
        Error::Msg(format!("Error creating directory {:?}", path))
    })?;
    let fname = format!("{}.feed", name);
    Ok(path.join(fname))
}

#[cfg(windows)]
pub fn get_config_path() -> Result<PathBuf, Error> {
    let path = app_data_dir()?;
    ::std::fs::create_dir_all(&path).map_err(|_| {
        Error::Msg(format!("Error creating directory {:?}", path))
    })?;
    Ok(path.join("config.feeds"))
}
