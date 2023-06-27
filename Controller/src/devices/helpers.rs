use super::{Device, DeviceWrapper, Id as DeviceId};
use crate::signals::{
    exchanger::{ConnectionRequested, DeviceIdSignalIdentifierBaseWrapper},
    Device as SignalsDevice, IdentifierBaseWrapper as SignalIdentifierBaseWrapper,
};
use std::{collections::HashMap, marker::PhantomData};

#[derive(Debug)]
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
            .collect()
    }
}

#[derive(Debug)]
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
        self.ds2ds(
            source_device.with_signal(source_signal_identifier),
            target_device.with_signal(target_signal_identifier),
        )
    }
    pub fn ds2ds<SD: Device + SignalsDevice, TD: Device + SignalsDevice>(
        &mut self,
        source: DeviceSignalHandle<SD>,
        target: DeviceSignalHandle<TD>,
    ) {
        self.dse2dse(source.into_erased(), target.into_erased());
    }
    pub fn dse2dse(
        &mut self,
        source: DeviceSignalHandleErased,
        target: DeviceSignalHandleErased,
    ) {
        self.connections_requested.push((
            source.into_device_id_signal_identifier_base_wrapper(),
            target.into_device_id_signal_identifier_base_wrapper(),
        ));
    }

    pub fn d2dse<SD: Device + SignalsDevice>(
        &mut self,
        source_device: DeviceHandle<SD>,
        source_signal_identifier: SD::Identifier,
        target: DeviceSignalHandleErased,
    ) {
        self.dse2dse(
            source_device
                .with_signal(source_signal_identifier)
                .into_erased(),
            target,
        );
    }
    pub fn dse2d<TD: Device + SignalsDevice>(
        &mut self,
        source: DeviceSignalHandleErased,
        target_device: DeviceHandle<TD>,
        target_signal_identifier: TD::Identifier,
    ) {
        self.dse2dse(
            source,
            target_device
                .with_signal(target_signal_identifier)
                .into_erased(),
        );
    }

    pub fn as_connections_requested(&self) -> &[ConnectionRequested] {
        &self.connections_requested
    }
    pub fn into_connections_requested(self) -> Box<[ConnectionRequested]> {
        self.connections_requested.into_boxed_slice()
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

    pub fn with_signal(
        self,
        signal_identifier: <D as SignalsDevice>::Identifier,
    ) -> DeviceSignalHandle<'d, D> {
        DeviceSignalHandle::new(self, signal_identifier)
    }

    pub fn into_erased(self) -> DeviceHandleErased<'d> {
        DeviceHandleErased::new(self.device_id)
    }
}
impl<'d, D: Device + SignalsDevice + 'd> Clone for DeviceHandle<'d, D> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'d, D: Device + SignalsDevice + 'd> Copy for DeviceHandle<'d, D> {}

#[derive(Clone, Debug)]
pub struct DeviceSignalHandle<'d, D: Device + SignalsDevice + 'd> {
    device_handle_erased: DeviceHandle<'d, D>,
    signal_identifier: <D as SignalsDevice>::Identifier,
}
impl<'d, D: Device + SignalsDevice + 'd> DeviceSignalHandle<'d, D> {
    pub fn new(
        device_handle_erased: DeviceHandle<'d, D>,
        signal_identifier: <D as SignalsDevice>::Identifier,
    ) -> Self {
        Self {
            device_handle_erased,
            signal_identifier,
        }
    }

    pub fn into_erased(self) -> DeviceSignalHandleErased<'d> {
        DeviceSignalHandleErased::new(
            self.device_handle_erased.into_erased(),
            SignalIdentifierBaseWrapper::new(self.signal_identifier.clone()),
        )
    }
}

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
}
impl<'d> Clone for DeviceHandleErased<'d> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'d> Copy for DeviceHandleErased<'d> {}

#[derive(Clone, Debug)]
pub struct DeviceSignalHandleErased<'d> {
    device_handle_erased: DeviceHandleErased<'d>,
    signal_identifier_base_wrapper: SignalIdentifierBaseWrapper,
}
impl<'d> DeviceSignalHandleErased<'d> {
    pub fn new(
        device_handle_erased: DeviceHandleErased<'d>,
        signal_identifier_base_wrapper: SignalIdentifierBaseWrapper,
    ) -> Self {
        Self {
            device_handle_erased,
            signal_identifier_base_wrapper,
        }
    }

    fn into_device_id_signal_identifier_base_wrapper(self) -> DeviceIdSignalIdentifierBaseWrapper {
        DeviceIdSignalIdentifierBaseWrapper::new(
            self.device_handle_erased.device_id(),
            self.signal_identifier_base_wrapper,
        )
    }
}
