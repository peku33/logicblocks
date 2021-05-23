use super::{Device, DeviceWrapper, Id as DeviceId};
use crate::signals::{
    exchange::{connections_requested::Connections, DeviceIdSignalId},
    Id as SignalId,
};
use std::collections::{HashMap, HashSet};

pub struct Devices<'d> {
    id_next: DeviceId,
    devices: HashMap<DeviceId, DeviceWrapper<'d>>,
}
impl<'d> Devices<'d> {
    pub fn new() -> Self {
        Self {
            id_next: 1,
            devices: HashMap::new(),
        }
    }
    pub fn device_add<N: ToString, D: Device + 'd>(
        &mut self,
        name: N,
        device: D,
    ) -> DeviceId {
        let name = name.to_string();
        let device = Box::new(device);

        let device_wrapper = DeviceWrapper::new(name, device);
        self.device_wrapper_add(device_wrapper)
    }
    pub fn device_wrapper_add(
        &mut self,
        device_wrapper: DeviceWrapper<'d>,
    ) -> DeviceId {
        let id = self.id_next;
        self.id_next += 1;

        let replaced = self.devices.insert(id, device_wrapper).is_some();
        assert!(!replaced);

        id
    }

    pub fn into_devices(self) -> HashMap<DeviceId, DeviceWrapper<'d>> {
        self.devices
    }
}

pub struct Signals {
    inner: Connections,
}
impl Signals {
    pub fn new() -> Self {
        let inner = Connections::new();

        Self { inner }
    }
    pub fn connect_disi(
        &mut self,
        source: DeviceIdSignalId,
        target: DeviceIdSignalId,
    ) {
        self.inner
            .entry(source)
            .or_insert_with(HashSet::new)
            .insert(target);
    }
    pub fn connect(
        &mut self,
        source_device_id: DeviceId,
        source_signal_id: SignalId,
        target_device_id: DeviceId,
        target_signal_id: SignalId,
    ) {
        self.connect_disi(
            DeviceIdSignalId {
                device_id: source_device_id,
                signal_id: source_signal_id,
            },
            DeviceIdSignalId {
                device_id: target_device_id,
                signal_id: target_signal_id,
            },
        )
    }
    pub fn into_signals(self) -> Connections {
        self.inner
    }
}
