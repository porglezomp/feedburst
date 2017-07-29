use std::env;
use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};

use error::Error;
use feed::FeedInfo;


#[derive(Debug, Clone)]
enum PathWrapper {
    CreateIfMissing(PathBuf),
    ErrorIfMissing(PathBuf),
}

#[derive(Clone)]
pub struct Args {
    only_fetch: bool,
    feed_root: Option<PathBuf>,
    config: PathWrapper,
}

impl Args {
    pub fn new(
        only_fetch: bool,
        feed_root: Option<&str>,
        config: Option<&str>,
    ) -> Result<Self, Error> {
        Ok(Args {
            only_fetch,
            feed_root: feed_root.map(From::from),
            config: config_path(config)?,
        })
    }

    pub fn config_path(&self) -> &PathBuf {
        match self.config {
            PathWrapper::CreateIfMissing(ref path) |
            PathWrapper::ErrorIfMissing(ref path) => path,
        }
    }

    pub fn config_file(&self) -> Result<File, Error> {
        match self.config {
            PathWrapper::CreateIfMissing(ref path) => {
                Ok(OpenOptions::new()
                    .create(true)
                    .write(true)
                    .read(true)
                    .open(path)
                    .map_err(|err| {
                        Error::Msg(format!("Cannot open file {:?}: {}", path, err))
                    })?)
            }
            PathWrapper::ErrorIfMissing(ref path) => {
                Ok(File::open(path).map_err(|err| {
                    Error::Msg(format!("Cannot open file {:?}: {}", path, err))
                })?)
            }

        }
    }

    pub fn feed_file(&self, info: &FeedInfo) -> Result<File, Error> {
        let root = self.feed_root.as_ref().or_else(|| info.root.as_ref());
        let path = feed_path(root, &info.name)?;
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .map_err(|err| {
                Error::Msg(format!("Error opening feed file {:?}: {}", path, err))
            })
    }
}

fn feed_path(root: Option<&PathBuf>, name: &str) -> Result<PathBuf, Error> {
    if let Some(root) = root {
        debug!("Using feed specified on the command line: {:?}", root);
        let root = Path::new(root);
        if !root.is_dir() {
            Err(Error::Msg(format!("Error: {:?} is not a directory", root)))
        } else {
            Ok(root.join(format!("{}.feed", name)))
        }
    } else {
        let path = platform_data_path(&format!("feeds/{}.feed", name))?;
        debug!("Using platform data: {:?}", path);
        Ok(path)
    }
}

fn config_path(path: Option<&str>) -> Result<PathWrapper, Error> {
    if let Some(path) = path {
        debug!("Using config specified on command line: {}", path);
        Ok(PathWrapper::ErrorIfMissing(path.into()))
    } else if let Some(path) = env::var_os("FEEDBURST_CONFIG_FILE") {
        debug!(
            "Using config specified as FEEDBURST_CONFIG_FILE: {}",
            path.to_string_lossy(),
        );
        Ok(PathWrapper::CreateIfMissing(path.into()))
    } else {
        let path = platform_config_path()?;
        debug!(
            "Using config found from the platform config dir: {:?}",
            path
        );
        Ok(PathWrapper::CreateIfMissing(path))
    }
}

#[cfg(unix)]
fn platform_data_path(path: &str) -> Result<PathBuf, Error> {
    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        Ok(path.into())
    } else {
        let xdg = ::xdg::BaseDirectories::with_prefix(::APP_NAME).map_err(
            |err| {
                Error::Msg(format!("{}", err))
            },
        )?;
        if let Some(path) = xdg.find_data_file(path) {
            Ok(path)
        } else {
            xdg.place_data_file(path).map_err(|err| {
                Error::Msg(format!("{}", err))
            })
        }
    }
}

#[cfg(unix)]
fn platform_config_path() -> Result<PathBuf, Error> {
    if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
        Ok(path.into())
    } else {
        let xdg = ::xdg::BaseDirectories::with_prefix(::APP_NAME).map_err(
            |err| {
                Error::Msg(format!("{}", err))
            },
        )?;
        if let Some(path) = xdg.find_config_file("config.feeds") {
            Ok(path)
        } else {
            xdg.place_config_file("config.feeds").map_err(|err| {
                Error::Msg(format!("{}", err))
            })
        }
    }
}


#[cfg(windows)]
fn app_data_dir() -> Result<PathBuf, Error> {
    if let Some(app_data_dir) = env::var_os("APPDATA") {
        Ok(Path::new(&app_data_dir).join("Feedburst"))
    } else {
        Err(Error::Msg("Unable to find the APPDATA directory".into()))
    }
}

#[cfg(windows)]
fn platform_data_path(path: &str) -> Result<PathBuf, Error> {
    let path = app_data_dir()?.join(path).parent().unwrap().into();
    ::std::fs::create_dir_all(&path).map_err(|err| {
        Error::Msg(format!(
            "Error creating feeds directory {:?}: {}",
            path,
            err
        ))
    })?;
    Ok(path)
}

#[cfg(windows)]
fn platform_config_path() -> Result<PathBuf, Error> {
    let path = app_data_dir()?;
    ::std::fs::create_dir_all(&path).map_err(|err| {
        Error::Msg(format!(
            "Error creating config directory {:?}: {}",
            path,
            err
        ))
    })?;
    Ok(path.join("config.feeds"))
}
