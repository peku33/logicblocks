#![cfg(target_os = "linux")]

use super::common::{Address, AddressDeviceType, AddressSerial, Frame, Payload};
use super::master::MasterDescriptor;
use failure::{err_msg, format_err, Error};
use futures::channel::oneshot;
use scopeguard::defer;
use std::cell::RefCell;
use std::convert::TryInto;
use std::ffi;
use std::fmt;
use std::fmt::{Debug, Display};
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

enum MasterTransaction {
    FrameOut {
        service_mode: bool,
        address: Address,
        out_payload: Payload,
        sender: oneshot::Sender<Result<(), Error>>,
    },
    FrameOutIn {
        service_mode: bool,
        address: Address,
        out_payload: Payload,
        in_timeout: Duration,
        sender: oneshot::Sender<Result<Payload, Error>>,
    },
    DeviceDiscovery {
        sender: oneshot::Sender<Result<Address, Error>>,
    },
}

pub struct MasterContext {
    ftdi_context: *mut libftdi1_sys::ftdi_context,
}
impl MasterContext {
    pub fn new() -> Result<Self, Error> {
        let ftdi_context_rc = RefCell::new(ptr::null_mut::<libftdi1_sys::ftdi_context>());

        defer! {{
            let ftdi_context = *ftdi_context_rc.borrow_mut();
            if !ftdi_context.is_null() {
                unsafe { libftdi1_sys::ftdi_free(ftdi_context) }
            }
        }}

        *ftdi_context_rc.borrow_mut() = unsafe { libftdi1_sys::ftdi_new() };
        let ftdi_context = *ftdi_context_rc.borrow_mut();
        if ftdi_context.is_null() {
            return Err(err_msg("ftdi_new() failed"));
        }

        Ok(Self {
            ftdi_context: ftdi_context_rc.replace(ptr::null_mut()),
        })
    }

    pub fn find_master_descriptors(&self) -> Result<Vec<MasterDescriptor>, Error> {
        let ftdi_device_list_ptr_rc =
            RefCell::new(ptr::null_mut::<libftdi1_sys::ftdi_device_list>());

        defer! {{
            let mut ftdi_device_list_ptr = *ftdi_device_list_ptr_rc.borrow_mut();
            if !ftdi_device_list_ptr.is_null() {
                unsafe { libftdi1_sys::ftdi_list_free(&mut ftdi_device_list_ptr) }
            }
        }}

        let mut ftdi_device_list_ptr = *ftdi_device_list_ptr_rc.borrow_mut();
        let ftdi_usb_find_all_result = unsafe {
            libftdi1_sys::ftdi_usb_find_all(self.ftdi_context, &mut ftdi_device_list_ptr, 0, 0)
        };
        if ftdi_usb_find_all_result < 0 {
            return Err(format_err!(
                "ftdi_usb_find_all() returned {}",
                ftdi_usb_find_all_result,
            ));
        }

        let mut master_descriptors =
            Vec::<MasterDescriptor>::with_capacity(ftdi_usb_find_all_result as usize);
        let mut ftdi_device_list_ptr_iter = ftdi_device_list_ptr;
        loop {
            if ftdi_device_list_ptr_iter.is_null() {
                break;
            }

            let libusb_device_ptr =
                unsafe { (*ftdi_device_list_ptr_iter).dev as *const libusb_sys::libusb_device };
            ftdi_device_list_ptr_iter = unsafe { (*ftdi_device_list_ptr_iter).next };

            // Get libusb_device_descriptor
            let mut libusb_device_descriptor =
                MaybeUninit::<libusb_sys::libusb_device_descriptor>::uninit();
            let libusb_get_device_descriptor_result = unsafe {
                libusb_sys::libusb_get_device_descriptor(
                    libusb_device_ptr,
                    libusb_device_descriptor.as_mut_ptr(),
                )
            };
            if libusb_get_device_descriptor_result != 0 {
                return Err(format_err!(
                    "libusb_get_device_descriptor() failed: {}",
                    libusb_get_device_descriptor_result,
                ));
            }
            let libusb_device_descriptor = unsafe { libusb_device_descriptor.assume_init() };

            // Check descriptors
            if libusb_device_descriptor.iSerialNumber == 0 {
                log::warn!(
                    "Missing iSerialNumber descriptor for device {:?} ({}:{})",
                    libusb_device_ptr,
                    libusb_device_descriptor.idVendor,
                    libusb_device_descriptor.idProduct,
                );
                continue;
            }

            // Open device
            let libusb_device_handle_ptr_rc =
                RefCell::new(ptr::null_mut::<libusb_sys::libusb_device_handle>());
            defer! {{
                let libusb_device_handle_ptr = *libusb_device_handle_ptr_rc.borrow_mut();
                if !libusb_device_handle_ptr.is_null() {
                    unsafe { libusb_sys::libusb_close(libusb_device_handle_ptr) }
                }
            }}
            let mut libusb_device_handle_ptr = *libusb_device_handle_ptr_rc.borrow_mut();
            let libusb_open_result = unsafe {
                libusb_sys::libusb_open(libusb_device_ptr, &mut libusb_device_handle_ptr)
            };
            if libusb_open_result != 0 {
                log::warn!(
                    "libusb_open() for {:?} ({}:{}) failed with status code {}",
                    libusb_device_ptr,
                    libusb_device_descriptor.idVendor,
                    libusb_device_descriptor.idProduct,
                    libusb_open_result
                );
                continue;
            }

            // Get serial
            let mut serial_number = [0u8; 128];
            let libusb_get_string_descriptor_ascii_serial_number_result = unsafe {
                libusb_sys::libusb_get_string_descriptor_ascii(
                    libusb_device_handle_ptr,
                    libusb_device_descriptor.iSerialNumber,
                    serial_number.as_mut_ptr(),
                    serial_number.len() as i32,
                )
            };
            if libusb_get_string_descriptor_ascii_serial_number_result <= 0 {
                log::warn!(
                    "Failed reading serial number for device {:?} ({}:{}) with status code {}",
                    libusb_device_ptr,
                    libusb_device_descriptor.idVendor,
                    libusb_device_descriptor.idProduct,
                    libusb_get_string_descriptor_ascii_serial_number_result,
                );
                continue;
            }
            let serial_number = ffi::CStr::from_bytes_with_nul(
                &serial_number
                    [..(libusb_get_string_descriptor_ascii_serial_number_result as usize) + 1],
            );
            let serial_number = match serial_number {
                Ok(serial_number) => serial_number,
                Err(e) => {
                    log::warn!(
                        "Failed decomposing serial number for device {:?} ({}:{}): {}",
                        libusb_device_ptr,
                        libusb_device_descriptor.idVendor,
                        libusb_device_descriptor.idProduct,
                        e,
                    );
                    continue;
                }
            };

            // Create MasterDescriptor
            let master_descriptor = MasterDescriptor {
                vid: libusb_device_descriptor.idVendor,
                pid: libusb_device_descriptor.idProduct,
                serial_number: serial_number.into(),
            };
            master_descriptors.push(master_descriptor);
        }
        Ok(master_descriptors)
    }
}
impl Drop for MasterContext {
    fn drop(&mut self) {
        unsafe { libftdi1_sys::ftdi_free(self.ftdi_context) }
    }
}

struct FtdiContextWrapper(*mut libftdi1_sys::ftdi_context);
unsafe impl Send for FtdiContextWrapper {
}

pub struct Master {
    master_descriptor: MasterDescriptor,

    ftdi_context: *mut libftdi1_sys::ftdi_context,
    worker_thread: Option<thread::JoinHandle<()>>, // Option to allow manual dropping
    worker_thread_sender: Option<mpsc::Sender<MasterTransaction>>, // Option to allow manual dropping
}
impl Master {
    pub fn new(master_descriptor: MasterDescriptor) -> Result<Self, Error> {
        let ftdi_context_rc = RefCell::new(ptr::null_mut::<libftdi1_sys::ftdi_context>());

        *ftdi_context_rc.borrow_mut() = unsafe { libftdi1_sys::ftdi_new() };

        defer! {{
            let ftdi_context = *ftdi_context_rc.borrow_mut();
            if !ftdi_context.is_null() {
                unsafe { libftdi1_sys::ftdi_free(ftdi_context) }
            }
        }}

        let ftdi_context = *ftdi_context_rc.borrow_mut();
        if ftdi_context.is_null() {
            return Err(err_msg("ftdi_new() failed"));
        }

        let ftdi_usb_open_desc_result = unsafe {
            libftdi1_sys::ftdi_usb_open_desc(
                ftdi_context,
                master_descriptor.vid as libc::c_int,
                master_descriptor.pid as libc::c_int,
                ptr::null(),
                master_descriptor.serial_number.as_ptr(),
            )
        };
        if ftdi_usb_open_desc_result != 0 {
            return Err(format_err!(
                "ftdi_usb_open_desc() failed with code {}",
                ftdi_usb_open_desc_result,
            ));
        }

        let ftdi_set_baudrate_result =
            unsafe { libftdi1_sys::ftdi_set_baudrate(ftdi_context, 115_200) };
        if ftdi_set_baudrate_result != 0 {
            return Err(format_err!(
                "ftdi_set_baudrate() failed with code {}",
                ftdi_set_baudrate_result,
            ));
        }

        let ftdi_set_line_property_result = unsafe {
            libftdi1_sys::ftdi_set_line_property2(
                ftdi_context,
                libftdi1_sys::ftdi_bits_type::BITS_7,
                libftdi1_sys::ftdi_stopbits_type::STOP_BIT_1,
                libftdi1_sys::ftdi_parity_type::EVEN,
                libftdi1_sys::ftdi_break_type::BREAK_OFF,
            )
        };
        if ftdi_set_line_property_result != 0 {
            return Err(format_err!(
                "ftdi_set_line_property() failed with code {}",
                ftdi_set_line_property_result,
            ));
        }

        let ftdi_setflowctrl_result = unsafe {
            libftdi1_sys::ftdi_setflowctrl(
                ftdi_context,
                0, // SIO_DISABLE_FLOW_CTRL
            )
        };
        if ftdi_setflowctrl_result != 0 {
            return Err(format_err!(
                "ftdi_setflowctrl() failed with code {}",
                ftdi_setflowctrl_result
            ));
        }

        let ftdi_set_latency_timer_result = unsafe {
            libftdi1_sys::ftdi_set_latency_timer(
                ftdi_context,
                1, // The lowest
            )
        };
        if ftdi_set_latency_timer_result != 0 {
            return Err(format_err!(
                "ftdi_set_latency_timer() failed with code {}",
                ftdi_set_latency_timer_result
            ));
        }

        let ftdi_context_wrapper = FtdiContextWrapper(ftdi_context);
        let (channel_sender, channel_receiver) = mpsc::channel::<MasterTransaction>();

        let worker_thread = thread::Builder::new()
            .name(format!(
                "Master({})",
                master_descriptor.serial_number.to_string_lossy()
            ))
            .spawn(move || {
                Self::thread_main(ftdi_context_wrapper.0, channel_receiver);
            })
            .unwrap();

        Ok(Self {
            master_descriptor,
            ftdi_context: ftdi_context_rc.replace(ptr::null_mut()),
            worker_thread: Some(worker_thread),
            worker_thread_sender: Some(channel_sender),
        })
    }

    pub async fn transaction_out(
        &self,

        service_mode: bool,
        address: Address,
        out_payload: Payload,
    ) -> Result<(), Error> {
        let (sender, receiver) = oneshot::channel();

        self.worker_thread_sender
            .as_ref()
            .unwrap()
            .clone()
            .send(MasterTransaction::FrameOut {
                service_mode,
                address,
                out_payload,
                sender,
            })
            .unwrap();

        receiver.await?
    }

    pub async fn transaction_out_in(
        &self,

        service_mode: bool,
        address: Address,
        out_payload: Payload,
        in_timeout: Duration,
    ) -> Result<Payload, Error> {
        let (sender, receiver) = oneshot::channel();

        self.worker_thread_sender
            .as_ref()
            .unwrap()
            .clone()
            .send(MasterTransaction::FrameOutIn {
                service_mode,
                address,
                out_payload,
                in_timeout,
                sender,
            })
            .unwrap();
        receiver.await?
    }

    pub async fn transaction_device_discovery(&self) -> Result<Address, Error> {
        let (sender, receiver) = oneshot::channel();

        self.worker_thread_sender
            .as_ref()
            .unwrap()
            .clone()
            .send(MasterTransaction::DeviceDiscovery { sender })
            .unwrap();

        receiver.await?
    }

    fn thread_main(
        ftdi_context: *mut libftdi1_sys::ftdi_context,
        receiver: mpsc::Receiver<MasterTransaction>,
    ) {
        for master_transaction in receiver.iter() {
            let send_result = match master_transaction {
                MasterTransaction::FrameOut {
                    service_mode,
                    address,
                    out_payload,
                    sender,
                } => sender
                    .send(Self::handle_transaction_frame_out(
                        ftdi_context,
                        service_mode,
                        &address,
                        &out_payload,
                    ))
                    .map_err(|e| e.map(|_| ())),

                MasterTransaction::FrameOutIn {
                    service_mode,
                    address,
                    out_payload,
                    in_timeout,
                    sender,
                } => sender
                    .send(Self::handle_transaction_frame_out_in(
                        ftdi_context,
                        service_mode,
                        &address,
                        &out_payload,
                        &in_timeout,
                    ))
                    .map_err(|e| e.map(|_| ())),

                MasterTransaction::DeviceDiscovery { sender } => sender
                    .send(Self::handle_transaction_device_discovery(
                        ftdi_context,
                        &Duration::from_millis(250),
                    ))
                    .map_err(|e| e.map(|_| ())),
            };
            send_result.unwrap_or_else(|error| log::warn!("Error while sending: {:?}", error));
        }
    }

    // Transaction handlers
    fn handle_transaction_frame_out(
        ftdi_context: *mut libftdi1_sys::ftdi_context,
        service_mode: bool,
        address: &Address,
        out_payload: &Payload,
    ) -> Result<(), Error> {
        Self::out_frame_phase(ftdi_context, service_mode, &address, &out_payload)?;
        Ok(())
    }

    fn handle_transaction_frame_out_in(
        ftdi_context: *mut libftdi1_sys::ftdi_context,
        service_mode: bool,
        address: &Address,
        out_payload: &Payload,
        in_timeout: &Duration,
    ) -> Result<Payload, Error> {
        Self::out_frame_phase(ftdi_context, service_mode, &address, &out_payload)?;
        let in_frame = Self::in_frame_phase(ftdi_context, service_mode, &address, &in_timeout)?;
        Ok(in_frame)
    }

    fn handle_transaction_device_discovery(
        ftdi_context: *mut libftdi1_sys::ftdi_context,
        in_timeout: &Duration,
    ) -> Result<Address, Error> {
        Self::out_device_discovery_phase(ftdi_context)?;
        let address = Self::in_device_discovery_phase(ftdi_context, in_timeout)?;
        Ok(address)
    }

    // Generic helpers
    fn out_phase(
        ftdi_context: *mut libftdi1_sys::ftdi_context,
        data: &[u8],
    ) -> Result<(), Error> {
        let ftdi_write_data_submit_result = unsafe {
            libftdi1_sys::ftdi_write_data_submit(
                ftdi_context,
                data.as_ptr() as *mut u8, // No idea why "write" takes non-const, it is not used mutably in lib
                data.len() as i32,
            )
        };
        if ftdi_write_data_submit_result.is_null() {
            return Err(err_msg("ftdi_write_data_submit() failed with NULL"));
        }

        let ftdi_transfer_data_done_result =
            unsafe { libftdi1_sys::ftdi_transfer_data_done(ftdi_write_data_submit_result) };
        if ftdi_transfer_data_done_result != data.len() as i32 {
            return Err(format_err!(
                "ftdi_transfer_data_done() failed with code {}",
                ftdi_transfer_data_done_result,
            ));
        }

        Ok(())
    }

    fn in_some_phase(
        ftdi_context: *mut libftdi1_sys::ftdi_context,
        timeout_left: &mut Duration,
    ) -> Result<Box<[u8]>, Error> {
        loop {
            let mut ftdi_read_data_buffer = [0, 128]; // FIXME: Move to heap buffer
            let ftdi_read_data_result = unsafe {
                libftdi1_sys::ftdi_read_data(
                    ftdi_context,
                    ftdi_read_data_buffer.as_mut_ptr(),
                    ftdi_read_data_buffer.len() as i32,
                )
            };
            if ftdi_read_data_result < 0 {
                return Err(format_err!(
                    "ftdi_read_data() failed with code {}",
                    ftdi_read_data_result,
                ));
            } else if ftdi_read_data_result == 0 {
                // No data was read, check the timeout

                // 1ms is the timeout of ftdi read op
                match timeout_left.checked_sub(Duration::from_millis(1)) {
                    Some(timeout_left_next) => *timeout_left = timeout_left_next,
                    None => return Err(err_msg("Timeout expired")),
                };
            } else {
                return Ok(Box::from(
                    &ftdi_read_data_buffer[0..ftdi_read_data_result as usize],
                ));
            }
        }
    }

    // Transaction helpers
    fn out_frame_phase(
        ftdi_context: *mut libftdi1_sys::ftdi_context,
        service_mode: bool,
        address: &Address,
        payload: &Payload,
    ) -> Result<(), Error> {
        let frame = Frame::out_build(service_mode, address, payload);
        Self::out_phase(ftdi_context, &frame)
    }

    fn in_frame_phase(
        ftdi_context: *mut libftdi1_sys::ftdi_context,
        service_mode: bool,
        address: &Address,
        timeout: &Duration,
    ) -> Result<Payload, Error> {
        const FRAME_BUFFER_MAX_LENGTH: usize = 1024;
        let mut frame_buffer = Vec::<u8>::new();

        let mut timeout_left = *timeout;
        loop {
            let frame = Self::in_some_phase(ftdi_context, &mut timeout_left)?;

            frame_buffer.extend_from_slice(&frame);
            if frame_buffer.len() > FRAME_BUFFER_MAX_LENGTH {
                return Err(err_msg("frame_buffer size exceeded. Noise?"));
            }

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
            )?;

            return Ok(payload);
        }
    }

    fn out_device_discovery_phase(
        ftdi_context: *mut libftdi1_sys::ftdi_context
    ) -> Result<(), Error> {
        Self::out_phase(ftdi_context, b"\x07")
    }

    fn in_device_discovery_phase(
        ftdi_context: *mut libftdi1_sys::ftdi_context,
        timeout: &Duration,
    ) -> Result<Address, Error> {
        const ADDRESS_LENGTH: usize = AddressSerial::LENGTH + AddressDeviceType::LENGTH;
        let mut frame_buffer = Vec::<u8>::new();

        let mut timeout_left = *timeout;
        loop {
            let frame = Self::in_some_phase(ftdi_context, &mut timeout_left)?;

            frame_buffer.extend_from_slice(&frame);
            if frame_buffer.len() > ADDRESS_LENGTH {
                return Err(err_msg("frame_buffer size exceeded. Noise?"));
            }

            if frame_buffer.len() == ADDRESS_LENGTH {
                let address_device_type =
                    AddressDeviceType::new(frame_buffer[0..AddressDeviceType::LENGTH].try_into()?)?;
                let address_serial = AddressSerial::new(
                    frame_buffer[AddressDeviceType::LENGTH..ADDRESS_LENGTH].try_into()?,
                )?;
                let address = Address::new(address_device_type, address_serial);
                return Ok(address);
            }
        }
    }
}
impl Display for Master {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        return write!(f, "Master({})", self.master_descriptor);
    }
}
impl Debug for Master {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        f.debug_struct("Master")
            .field("master_descriptor", &self.master_descriptor)
            .finish()
    }
}
impl Drop for Master {
    fn drop(&mut self) {
        self.worker_thread_sender.take(); // Closes the pipe, effectively telling thread to stop
        self.worker_thread.take().unwrap().join().unwrap(); // Close the thread

        if !self.ftdi_context.is_null() {
            unsafe { libftdi1_sys::ftdi_free(self.ftdi_context) }
        }
    }
}
