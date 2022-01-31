use super::{Device, DeviceWrapper, Id as DeviceId};
use crate::signals::{
    exchanger::{ConnectionRequested, DeviceIdSignalIdentifierBaseWrapper},
    Device as SignalsDevice, Identifier as SignalIdentifier,
    IdentifierBaseWrapper as SignalIdentifierBaseWrapper,
};
use std::{collections::HashMap, marker::PhantomData};

pub struct Devices<'d> {
    device_wrappers: Vec<DeviceWrapper<'d>>,
}
impl<'d> Devices<'d> {
    pub fn new() -> Self {
        Self {
            device_wrappers: Vec::<DeviceWrapper<'d>>::new(),
        }
    }

    pub fn add<N: ToString, D: Device + SignalsDevice + 'd>(
        &mut self,
        name: N,
        device: D,
    ) -> DeviceHandle<'d, D> {
        let name = name.to_string();
        let device = Box::new(device);

        let device_wrapper = DeviceWrapper::new(name, device);

        let device_id = (self.device_wrappers.len() + 1) as DeviceId; // starts from 1
        self.device_wrappers.push(device_wrapper);

        DeviceHandle::<D>::new(device_id)
    }

    pub fn into_device_wrappers_by_id(self) -> HashMap<DeviceId, DeviceWrapper<'d>> {
        self.device_wrappers
            .into_iter()
            .enumerate()
            .map(|(device_id, device)| ((device_id + 1) as u32, device))
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

    // device to device
    pub fn d2d<SD: Device + SignalsDevice, TD: Device + SignalsDevice>(
        &mut self,
        source_device: DeviceHandle<SD>,
        source_signal_identifier: SD::Identifier,
        target_device: DeviceHandle<TD>,
        target_signal_identifier: TD::Identifier,
    ) {
        self.s2s(
            source_device.signal(source_signal_identifier),
            target_device.signal(target_signal_identifier),
        );
    }
    // device erased to device erased
    pub fn de2de<S: SignalIdentifier, T: SignalIdentifier>(
        &mut self,
        source_device: DeviceHandleErased,
        source_signal_identifier: S,
        target_device: DeviceHandleErased,
        target_signal_identifier: S,
    ) {
        self.s2s(
            source_device.signal(source_signal_identifier),
            target_device.signal(target_signal_identifier),
        )
    }
    // signal to signal
    pub fn s2s(
        &mut self,
        source: DeviceIdSignalIdentifierBaseWrapper,
        target: DeviceIdSignalIdentifierBaseWrapper,
    ) {
        self.connections_requested.push((source, target));
    }

    pub fn d2s<SD: Device + SignalsDevice>(
        &mut self,
        source_device: DeviceHandle<SD>,
        source_signal_identifier: SD::Identifier,
        target: DeviceIdSignalIdentifierBaseWrapper,
    ) {
        self.s2s(source_device.signal(source_signal_identifier), target);
    }
    pub fn s2d<TD: Device + SignalsDevice>(
        &mut self,
        source: DeviceIdSignalIdentifierBaseWrapper,
        target_device: DeviceHandle<TD>,
        target_signal_identifier: TD::Identifier,
    ) {
        self.s2s(source, target_device.signal(target_signal_identifier));
    }

    pub fn as_connections_requested(&self) -> &[ConnectionRequested] {
        self.connections_requested.as_slice()
    }
    pub fn into_connections_requested(self) -> Vec<ConnectionRequested> {
        self.connections_requested
    }
}

#[derive(Debug)]
pub struct DeviceHandle<'d, D: Device + SignalsDevice + 'd> {
    device_id: DeviceId,

    phantom_data: PhantomData<&'d D>,
}
impl<'d, D: Device + SignalsDevice + 'd> DeviceHandle<'d, D> {
    fn new(device_id: DeviceId) -> Self {
        Self {
            device_id,

            phantom_data: PhantomData,
        }
    }

    pub fn signal(
        &self,
        signal_identifier: D::Identifier,
    ) -> DeviceIdSignalIdentifierBaseWrapper {
        let signal_identifier_base_wrapper = SignalIdentifierBaseWrapper::new(signal_identifier);
        let device_id_signal_identifier_base_wrapper = DeviceIdSignalIdentifierBaseWrapper::new(
            self.device_id,
            signal_identifier_base_wrapper,
        );
        device_id_signal_identifier_base_wrapper
    }

    pub fn erased(&self) -> DeviceHandleErased<'d> {
        DeviceHandleErased::new(self.device_id)
    }
}
impl<'d, D: Device + SignalsDevice + 'd> Clone for DeviceHandle<'d, D> {
    fn clone(&self) -> Self {
        Self { ..*self }
    }
}
impl<'d, D: Device + SignalsDevice + 'd> Copy for DeviceHandle<'d, D> {}

#[derive(Debug)]
pub struct DeviceHandleErased<'d> {
    device_id: DeviceId,

    phantom_data: PhantomData<&'d ()>,
}
impl<'d> DeviceHandleErased<'d> {
    fn new(device_id: DeviceId) -> Self {
        Self {
            device_id,
            phantom_data: PhantomData,
        }
    }

    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub fn signal<I: SignalIdentifier>(
        &self,
        signal_identifier: I,
    ) -> DeviceIdSignalIdentifierBaseWrapper {
        let signal_identifier_base_wrapper = SignalIdentifierBaseWrapper::new(signal_identifier);
        let device_id_signal_identifier_base_wrapper = DeviceIdSignalIdentifierBaseWrapper::new(
            self.device_id,
            signal_identifier_base_wrapper,
        );
        device_id_signal_identifier_base_wrapper
    }
}
impl<'d> Clone for DeviceHandleErased<'d> {
    fn clone(&self) -> Self {
        Self { ..*self }
    }
}
impl<'d> Copy for DeviceHandleErased<'d> {}
