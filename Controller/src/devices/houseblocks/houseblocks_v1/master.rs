#[cfg(target_os = "linux")]
pub use super::master_linux::*;

#[cfg(not(target_os = "linux"))]
pub use super::master_stub::*;

use std::{
    ffi, fmt,
    fmt::{Debug, Display},
};

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct MasterDescriptor {
    pub vid: u16,
    pub pid: u16,
    pub serial_number: ffi::CString,
}
impl Display for MasterDescriptor {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        return write!(
            f,
            "{:04X}:{:04X}:{}",
            self.vid,
            self.pid,
            self.serial_number.to_string_lossy()
        );
    }
}
