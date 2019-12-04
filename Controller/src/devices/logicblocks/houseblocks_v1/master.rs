#[cfg(target_os = "linux")]
pub use super::master_linux::*;

#[cfg(not(target_os = "linux"))]
pub use super::master_stub::*;
