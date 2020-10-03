pub mod connections_map;
pub mod connections_requested;
pub mod connections_running;
pub mod exchanger;

use super::Id as SignalId;
use crate::devices::Id as DeviceId;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct DeviceIdSignalId {
    pub device_id: DeviceId,
    pub signal_id: SignalId,
}
