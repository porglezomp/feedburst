#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::open_url;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use self::windows::{config_path, data_path, open_url};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::open_url;

/// For code that's the same on macOS and Linux
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::{config_path, data_path};
