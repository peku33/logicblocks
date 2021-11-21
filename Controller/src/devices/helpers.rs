use super::{Device, DeviceWrapper, Id as DeviceId};
use crate::signals::{
    exchanger::ConnectionRequested, Device as SignalsDevice, IdentifierBaseWrapper,
};
use std::{collections::HashMap, marker::PhantomData};

pub struct DevicesItem<'d, D: Device + SignalsDevice + 'd> {
    device: PhantomData<&'d D>,
    id: DeviceId,
}
impl<'d, D: Device + SignalsDevice + 'd> Copy for DevicesItem<'d, D> {}
impl<'d, D: Device + SignalsDevice + 'd> Clone for DevicesItem<'d, D> {
    fn clone(&self) -> Self {
        Self { ..*self }
    }
}

pub struct Devices<'d> {
    device_wrappers: Vec<DeviceWrapper<'d>>,
}
impl<'d> Devices<'d> {
    pub fn new() -> Self {
        Self {
            device_wrappers: Vec::new(),
        }
    }
    pub fn device_add<N: ToString, D: Device + SignalsDevice + 'd>(
        &mut self,
        name: N,
        device: D,
    ) -> DevicesItem<'d, D> {
        let name = name.to_string();
        let device = Box::new(device);

        let device_wrapper = DeviceWrapper::new(name, device);

        let id = self.device_wrappers.len();
        self.device_wrappers.push(device_wrapper);

        DevicesItem {
            device: PhantomData,
            id: id as u32,
        }
    }
    pub fn into_device_wrappers_by_id(self) -> HashMap<DeviceId, DeviceWrapper<'d>> {
        self.device_wrappers
            .into_iter()
            .enumerate()
            .map(|(device_id, device)| (device_id as u32, device))
            .collect::<HashMap<_, _>>()
    }
}

pub struct Signals {
    connections_requested: Vec<ConnectionRequested>,
}
impl Signals {
    pub fn new() -> Self {
        let connections_requested = Vec::<ConnectionRequested>::new();

        Self {
            connections_requested,
        }
    }

    pub fn connect<SD: Device + SignalsDevice, TD: Device + SignalsDevice>(
        &mut self,
        source_device_item: DevicesItem<SD>,
        source_signal_identifier: SD::Identifier,
        target_device_item: DevicesItem<TD>,
        target_signal_identifier: TD::Identifier,
    ) {
        self.connections_requested.push((
            (
                source_device_item.id,
                IdentifierBaseWrapper::new(source_signal_identifier),
            ),
            (
                target_device_item.id,
                IdentifierBaseWrapper::new(target_signal_identifier),
            ),
        ));
    }

    pub fn as_connections_requested(&self) -> &[ConnectionRequested] {
        self.connections_requested.as_slice()
    }
    pub fn into_connections_requested(self) -> Vec<ConnectionRequested> {
        self.connections_requested
    }
}
