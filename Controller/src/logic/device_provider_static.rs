use super::{
    device::Device,
    device_provider::{DeviceId, DeviceProvider},
};
use futures::stream::{pending, BoxStream, StreamExt};
use std::{collections::HashSet, fmt, sync::Mutex};

pub struct DeviceProviderStatic {
    devices: Mutex<Vec<Option<Box<dyn Device>>>>,
}
impl DeviceProviderStatic {
    pub fn new(devices: Vec<Box<dyn Device>>) -> Self {
        Self {
            devices: Mutex::new(devices.into_iter().map(Some).collect()),
        }
    }
}
impl fmt::Debug for DeviceProviderStatic {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> Result<(), fmt::Error> {
        f.debug_struct("DeviceProviderStatic").finish()
    }
}
impl DeviceProvider for DeviceProviderStatic {
    fn get_change_stream(&self) -> BoxStream<()> {
        pending().boxed()
    }
    fn get_device_ids(&self) -> HashSet<DeviceId> {
        let devices = self.devices.lock().unwrap();
        let devices_len = devices.len();
        drop(devices);
        (0..devices_len as DeviceId).collect()
    }
    fn get_device(
        &self,
        device_id: DeviceId,
    ) -> Option<Box<dyn Device>> {
        let mut devices = self.devices.lock().unwrap();
        let device = devices.get_mut(device_id as usize)?.take().unwrap();
        drop(devices);
        Some(device)
    }
}
