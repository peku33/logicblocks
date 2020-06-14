pub mod common;
pub mod master;

#[cfg(target_os = "linux")]
pub mod master_linux;

#[cfg(not(target_os = "linux"))]
pub mod master_stub;
