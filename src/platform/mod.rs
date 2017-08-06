#[cfg(target_os = "linux")]
mod linux;
// #[cfg(target_os = "linux")]
// pub use self::linux::{};

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use self::windows::{data_path, config_path};

#[cfg(target_os = "macos")]
mod macos;
// #[cfg(target_os = "macos")]
// pub use self::macos::{};

/// For code that's the same on macOS and Linux
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::{data_path, config_path};
