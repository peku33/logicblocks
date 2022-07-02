use super::common::{Address, AddressDeviceType, AddressSerial, Frame, Payload};
use crate::interfaces::serial::{
    ftdi::{
        Descriptor as FtdiDescriptor, DeviceConfiguration as FtdiDeviceConfiguration,
        DeviceFailSafe as FtdiDeviceFailSafe,
    },
    Bits, Configuration as SerialConfiguration, Parity, StopBits,
};
use anyhow::{bail, ensure, Context, Error};
use crossbeam::channel;
use futures::channel::oneshot;
use std::{fmt::Debug, mem::ManuallyDrop, thread, time::Duration};

#[derive(Debug)]
enum Transaction {
    FrameOut {
        service_mode: bool,
        address: Address,
        out_payload: Payload,
        result_sender: oneshot::Sender<Result<(), Error>>,
    },
    FrameOutIn {
        service_mode: bool,
        address: Address,
        out_payload: Payload,
        in_timeout: Duration,
        result_sender: oneshot::Sender<Result<Payload, Error>>,
    },
    DeviceDiscovery {
        result_sender: oneshot::Sender<Result<Address, Error>>,
    },
}

struct Driver {
    ftdi_device: FtdiDeviceFailSafe,
}
impl Driver {
    const SERIAL_CONFIGURATION: SerialConfiguration = SerialConfiguration {
        baud_rate: 115_200,
        bits: Bits::Bits7,
        stop_bits: StopBits::StopBits1,
        parity: Parity::Even,
    };
    const FTDI_DEVICE_CONFIGURATION: FtdiDeviceConfiguration = FtdiDeviceConfiguration {
        latency_timer_ms: 10,
    };

    pub fn new(ftdi_descriptor: FtdiDescriptor) -> Self {
        Self {
            ftdi_device: FtdiDeviceFailSafe::new(
                ftdi_descriptor,
                Self::SERIAL_CONFIGURATION,
                Self::FTDI_DEVICE_CONFIGURATION,
                3,
                Duration::from_secs(1),
            ),
        }
    }

    fn phase_frame_out(
        &mut self,
        service_mode: bool,
        address: &Address,
        payload: &Payload,
    ) -> Result<(), Error> {
        let frame = Frame::out_build(service_mode, address, payload);
        self.ftdi_device.write(&frame).context("write")?;
        Ok(())
    }
    fn phase_frame_in(
        &mut self,
        service_mode: bool,
        address: &Address,
        timeout: &Duration,
    ) -> Result<Payload, Error> {
        const FRAME_BUFFER_MAX_LENGTH: usize = 1024;
        let mut frame_buffer = Vec::<u8>::new();

        let mut timeout_left = *timeout;
        loop {
            let frame = self.ftdi_device.read().context("read")?;
            if frame.is_empty() {
                match timeout_left.checked_sub(Duration::from_millis(
                    Self::FTDI_DEVICE_CONFIGURATION.latency_timer_ms as u64,
                )) {
                    Some(timeout_left_next) => {
                        timeout_left = timeout_left_next;
                    }
                    None => bail!("timeout expired"),
                }
                continue;
            }

            frame_buffer.extend_from_slice(&frame);
            ensure!(
                frame_buffer.len() <= FRAME_BUFFER_MAX_LENGTH,
                "frame_buffer size exceeded. Noise?"
            );

            let char_begin_position = match frame_buffer
                .iter()
                .position(|item| *item == Frame::CHAR_BEGIN)
            {
                Some(char_begin_position) => char_begin_position,
                None => continue,
            };
            if char_begin_position != 0 {
                log::warn!("Frame::CHAR_BEGIN not on beginning of message. Noise?");
            }

            let char_end_position = match frame_buffer[char_begin_position..]
                .iter()
                .position(|item| *item == Frame::CHAR_END)
                .map(|position| position + char_begin_position)
            {
                Some(char_end_position) => char_end_position,
                None => continue,
            };
            if char_end_position != frame_buffer.len() - 1 {
                log::warn!("Frame::CHAR_END not on end of message. Noise?");
            }

            let payload = Frame::in_parse(
                &frame_buffer[char_begin_position..char_end_position + 1],
                service_mode,
                address,
            )
            .context("payload")?;

            return Ok(payload);
        }
    }

    fn phase_device_discovery_out(&mut self) -> Result<(), Error> {
        self.ftdi_device.write(b"\x07").context("write")?;
        Ok(())
    }
    fn phase_device_discovery_in(
        &mut self,
        timeout: &Duration,
    ) -> Result<Address, Error> {
        const ADDRESS_LENGTH: usize = AddressSerial::LENGTH + AddressDeviceType::LENGTH;
        let mut frame_buffer = Vec::<u8>::new();

        let mut timeout_left = *timeout;
        loop {
            let frame = self.ftdi_device.read().context("read")?;
            if frame.is_empty() {
                match timeout_left.checked_sub(Duration::from_millis(
                    Self::FTDI_DEVICE_CONFIGURATION.latency_timer_ms as u64,
                )) {
                    Some(timeout_left_next) => {
                        timeout_left = timeout_left_next;
                    }
                    None => bail!("timeout expired"),
                }
                continue;
            }

            frame_buffer.extend_from_slice(&frame);
            ensure!(
                frame_buffer.len() <= ADDRESS_LENGTH,
                "frame_buffer size exceeded. Noise?"
            );

            if frame_buffer.len() == ADDRESS_LENGTH {
                let address_device_type = AddressDeviceType::new(
                    frame_buffer[0..AddressDeviceType::LENGTH]
                        .try_into()
                        .unwrap(),
                )
                .context("address_device_type")?;
                let address_serial = AddressSerial::new(
                    frame_buffer[AddressDeviceType::LENGTH..ADDRESS_LENGTH]
                        .try_into()
                        .unwrap(),
                )
                .context("address_serial")?;
                let address = Address {
                    device_type: address_device_type,
                    serial: address_serial,
                };
                return Ok(address);
            }
        }
    }

    pub fn transaction_frame_out(
        &mut self,
        service_mode: bool,
        address: &Address,
        out_payload: &Payload,
    ) -> Result<(), Error> {
        self.phase_frame_out(service_mode, address, out_payload)
            .context("phase_frame_out")?;
        Ok(())
    }
    pub fn transaction_frame_out_in(
        &mut self,
        service_mode: bool,
        address: &Address,
        out_payload: &Payload,
        in_timeout: &Duration,
    ) -> Result<Payload, Error> {
        self.phase_frame_out(service_mode, address, out_payload)
            .context("phase_frame_out")?;
        let in_frame = self
            .phase_frame_in(service_mode, address, in_timeout)
            .context("phase_frame_in")?;
        Ok(in_frame)
    }
    pub fn transaction_device_discovery(
        &mut self,
        in_timeout: &Duration,
    ) -> Result<Address, Error> {
        self.ftdi_device.purge().context("purge")?;
        self.phase_device_discovery_out()
            .context("phase_device_discovery_out")?;
        let address = self
            .phase_device_discovery_in(in_timeout)
            .context("phase_device_discovery_in")?;
        Ok(address)
    }
}

#[derive(Debug)]
pub struct Master {
    ftdi_descriptor: FtdiDescriptor,

    transaction_sender: ManuallyDrop<channel::Sender<Transaction>>,
    worker_thread: ManuallyDrop<thread::JoinHandle<()>>,
}
impl Master {
    pub fn new(ftdi_descriptor: FtdiDescriptor) -> Self {
        let (transaction_sender, transaction_receiver) = channel::unbounded::<Transaction>();

        let worker_ftdi_descriptor = ftdi_descriptor.clone();
        let worker_thread = thread::Builder::new()
            .name(format!(
                "{}.houseblocks_v1.master",
                ftdi_descriptor.serial_number.to_str().unwrap()
            ))
            .spawn(move || {
                Self::thread_main(worker_ftdi_descriptor, transaction_receiver);
            })
            .unwrap();

        Self {
            ftdi_descriptor,
            transaction_sender: ManuallyDrop::new(transaction_sender),
            worker_thread: ManuallyDrop::new(worker_thread),
        }
    }

    pub async fn transaction_out(
        &self,

        service_mode: bool,
        address: Address,
        out_payload: Payload,
    ) -> Result<(), Error> {
        let (result_sender, result_receiver) = oneshot::channel::<Result<(), Error>>();

        self.transaction_sender
            .send(Transaction::FrameOut {
                service_mode,
                address,
                out_payload,
                result_sender,
            })
            .unwrap();

        result_receiver.await.unwrap().context("result_receiver")?;
        Ok(())
    }
    pub async fn transaction_out_in(
        &self,

        service_mode: bool,
        address: Address,
        out_payload: Payload,
        in_timeout: Duration,
    ) -> Result<Payload, Error> {
        let (result_sender, result_receiver) = oneshot::channel::<Result<Payload, Error>>();

        self.transaction_sender
            .send(Transaction::FrameOutIn {
                service_mode,
                address,
                out_payload,
                in_timeout,
                result_sender,
            })
            .unwrap();

        let result = result_receiver.await.unwrap().context("result_receiver")?;
        Ok(result)
    }
    pub async fn transaction_device_discovery(&self) -> Result<Address, Error> {
        let (result_sender, result_receiver) = oneshot::channel::<Result<Address, Error>>();

        self.transaction_sender
            .send(Transaction::DeviceDiscovery { result_sender })
            .unwrap();

        result_receiver.await.context("result_receiver")?
    }

    fn thread_main(
        ftdi_descriptor: FtdiDescriptor,
        transaction_receiver: channel::Receiver<Transaction>,
    ) {
        let mut driver = Driver::new(ftdi_descriptor);

        for transaction in transaction_receiver.iter() {
            let _ = match transaction {
                Transaction::FrameOut {
                    service_mode,
                    address,
                    out_payload,
                    result_sender,
                } => result_sender
                    .send(driver.transaction_frame_out(service_mode, &address, &out_payload))
                    .map_err(|e| e.map(|_| ())),

                Transaction::FrameOutIn {
                    service_mode,
                    address,
                    out_payload,
                    in_timeout,
                    result_sender,
                } => result_sender
                    .send(driver.transaction_frame_out_in(
                        service_mode,
                        &address,
                        &out_payload,
                        &in_timeout,
                    ))
                    .map_err(|e| e.map(|_| ())),

                Transaction::DeviceDiscovery { result_sender } => result_sender
                    .send(driver.transaction_device_discovery(&Duration::from_millis(250)))
                    .map_err(|e| e.map(|_| ())),
            };
        }
    }
}
impl Drop for Master {
    fn drop(&mut self) {
        // This ends the iteration
        unsafe { ManuallyDrop::drop(&mut self.transaction_sender) };

        // This joins and awaits the thread
        unsafe { ManuallyDrop::take(&mut self.worker_thread) }
            .join()
            .unwrap();
    }
}
