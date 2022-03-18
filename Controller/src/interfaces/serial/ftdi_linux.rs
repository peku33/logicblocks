#![cfg(target_os = "linux")]

use super::{
    ftdi::{Descriptor, Descriptors, DeviceConfiguration},
    Bits, Configuration, Parity, StopBits,
};
use anyhow::{bail, ensure, Context, Error};
use libftdi1_sys::*;
use scopeguard::defer;
use std::{cell::RefCell, ffi, mem::MaybeUninit, ptr};

pub struct Global {
    context: *mut ftdi_context,
}
impl Global {
    pub fn new() -> Result<Self, Error> {
        let context_rc = RefCell::new(ptr::null_mut::<ftdi_context>());

        defer! {{
            let context = *context_rc.borrow_mut();
            if !context.is_null() {
                unsafe { ftdi_free(context) }
            }
        }}

        *context_rc.borrow_mut() = unsafe { ftdi_new() };
        let context = *context_rc.borrow_mut();
        ensure!(!context.is_null(), "ftdi_new() failed");

        Ok(Self {
            context: context_rc.replace(ptr::null_mut()),
        })
    }

    pub fn find_descriptors(&mut self) -> Result<Descriptors, Error> {
        let ftdi_device_list_ptr_rc = RefCell::new(ptr::null_mut::<ftdi_device_list>());

        defer! {{
            let mut ftdi_device_list_ptr = *ftdi_device_list_ptr_rc.borrow_mut();
            if !ftdi_device_list_ptr.is_null() {
                unsafe { ftdi_list_free(&mut ftdi_device_list_ptr) }
            }
        }}

        let mut ftdi_device_list_ptr = *ftdi_device_list_ptr_rc.borrow_mut();
        let ftdi_usb_find_all_result =
            unsafe { ftdi_usb_find_all(self.context, &mut ftdi_device_list_ptr, 0, 0) };
        ensure!(
            ftdi_usb_find_all_result >= 0,
            "ftdi_usb_find_all() returned {}",
            ftdi_usb_find_all_result
        );

        let mut descriptors = Vec::<Descriptor>::with_capacity(ftdi_usb_find_all_result as usize);
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
            ensure!(
                libusb_get_device_descriptor_result == 0,
                "libusb_get_device_descriptor() failed: {}",
                libusb_get_device_descriptor_result,
            );
            let libusb_device_descriptor = unsafe { libusb_device_descriptor.assume_init() };

            // Check descriptors
            if libusb_device_descriptor.iSerialNumber == 0 {
                log::warn!(
                    "missing iSerialNumber descriptor for device {:?} ({}:{})",
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
                    "failed reading serial number for device {:?} ({}:{}) with status code {}",
                    libusb_device_ptr,
                    libusb_device_descriptor.idVendor,
                    libusb_device_descriptor.idProduct,
                    libusb_get_string_descriptor_ascii_serial_number_result,
                );
                continue;
            }
            let serial_number = match ffi::CStr::from_bytes_with_nul(
                &serial_number
                    [..(libusb_get_string_descriptor_ascii_serial_number_result as usize) + 1],
            )
            .context("serial_number")
            {
                Ok(serial_number) => serial_number,
                Err(error) => {
                    log::warn!(
                        "failed decomposing serial number for device {:?} ({}:{}): {:?}",
                        libusb_device_ptr,
                        libusb_device_descriptor.idVendor,
                        libusb_device_descriptor.idProduct,
                        error,
                    );
                    continue;
                }
            };

            // Create Descriptor
            let descriptor = Descriptor {
                vid: libusb_device_descriptor.idVendor,
                pid: libusb_device_descriptor.idProduct,
                serial_number: serial_number.into(),
            };
            descriptors.push(descriptor);
        }
        let descriptors = Descriptors::new(descriptors);
        Ok(descriptors)
    }
}
impl Drop for Global {
    fn drop(&mut self) {
        unsafe { ftdi_free(self.context) }
    }
}

pub struct Device {
    context: *mut ftdi_context,
}
impl Device {
    pub fn new(
        descriptor: &Descriptor,
        configuration: &Configuration,
        device_configuration: &DeviceConfiguration,
    ) -> Result<Self, Error> {
        let context_rc = RefCell::new(ptr::null_mut::<ftdi_context>());

        *context_rc.borrow_mut() = unsafe { ftdi_new() };

        defer! {{
            let context = *context_rc.borrow_mut();
            if !context.is_null() {
                unsafe { ftdi_free(context) }
            }
        }}

        let context = *context_rc.borrow_mut();
        ensure!(!context.is_null(), "ftdi_new() failed");

        let ftdi_usb_open_desc_result = unsafe {
            ftdi_usb_open_desc(
                context,
                descriptor.vid as libc::c_int,
                descriptor.pid as libc::c_int,
                ptr::null(),
                descriptor.serial_number.as_ptr(),
            )
        };
        ensure!(
            ftdi_usb_open_desc_result == 0,
            "ftdi_usb_open_desc() failed with code {}",
            ftdi_usb_open_desc_result,
        );

        let ftdi_set_baudrate_result =
            unsafe { ftdi_set_baudrate(context, configuration.baud_rate as i32) };
        ensure!(
            ftdi_set_baudrate_result == 0,
            "ftdi_set_baudrate() failed with code {}",
            ftdi_set_baudrate_result,
        );

        let ftdi_set_line_property_result = unsafe {
            ftdi_set_line_property2(
                context,
                make_ftdi_bits_type(&configuration.bits),
                make_ftdi_stopbits_type(&configuration.stop_bits),
                make_ftdi_parity_type(&configuration.parity),
                ftdi_break_type::BREAK_OFF,
            )
        };
        ensure!(
            ftdi_set_line_property_result == 0,
            "ftdi_set_line_property() failed with code {}",
            ftdi_set_line_property_result,
        );

        let ftdi_setflowctrl_result = unsafe {
            ftdi_setflowctrl(
                context, 0, // SIO_DISABLE_FLOW_CTRL
            )
        };
        ensure!(
            ftdi_setflowctrl_result == 0,
            "ftdi_setflowctrl() failed with code {}",
            ftdi_setflowctrl_result
        );

        let ftdi_set_latency_timer_result =
            unsafe { ftdi_set_latency_timer(context, device_configuration.latency_timer_ms) };
        ensure!(
            ftdi_set_latency_timer_result == 0,
            "ftdi_set_latency_timer() failed with code {}",
            ftdi_set_latency_timer_result
        );

        Ok(Self {
            context: context_rc.replace(ptr::null_mut()),
        })
    }

    pub fn purge(&mut self) -> Result<(), Error> {
        let ftdi_usb_purge_buffers_result = unsafe { ftdi_usb_purge_buffers(self.context) };
        ensure!(
            ftdi_usb_purge_buffers_result == 0,
            "ftdi_usb_purge_buffers() failed with code {}",
            ftdi_usb_purge_buffers_result,
        );
        Ok(())
    }
    pub fn write(
        &mut self,
        data: &[u8],
    ) -> Result<(), Error> {
        let ftdi_write_data_submit_result = unsafe {
            ftdi_write_data_submit(
                self.context,
                data.as_ptr() as *mut u8, // No idea why "write" takes non-const, it is not used mutably in lib
                data.len() as i32,
            )
        };
        ensure!(
            !ftdi_write_data_submit_result.is_null(),
            "ftdi_write_data_submit() failed with NULL"
        );

        let ftdi_transfer_data_done_result =
            unsafe { ftdi_transfer_data_done(ftdi_write_data_submit_result) };
        ensure!(
            ftdi_transfer_data_done_result == data.len() as i32,
            "ftdi_transfer_data_done() failed with code {}",
            ftdi_transfer_data_done_result,
        );

        Ok(())
    }
    pub fn read(&mut self) -> Result<Box<[u8]>, Error> {
        let mut ftdi_read_data_buffer = [0, 128]; // TODO: Move to heap buffer
        let ftdi_read_data_result = unsafe {
            ftdi_read_data(
                self.context,
                ftdi_read_data_buffer.as_mut_ptr(),
                ftdi_read_data_buffer.len() as i32,
            )
        };

        if ftdi_read_data_result < 0 {
            bail!(
                "ftdi_read_data() failed with code {}",
                ftdi_read_data_result,
            );
        } else if ftdi_read_data_result == 0 {
            Ok(Vec::<u8>::new().into_boxed_slice())
        } else {
            Ok(Box::from(
                &ftdi_read_data_buffer[0..ftdi_read_data_result as usize],
            ))
        }
    }
}
impl Drop for Device {
    fn drop(&mut self) {
        if !self.context.is_null() {
            unsafe { ftdi_free(self.context) }
        }
    }
}

fn make_ftdi_bits_type(bits: &Bits) -> ftdi_bits_type {
    match bits {
        Bits::Bits7 => ftdi_bits_type::BITS_7,
        Bits::Bits8 => ftdi_bits_type::BITS_8,
        #[allow(unreachable_patterns)]
        _ => panic!("not supported Bits: {:?}", bits),
    }
}
fn make_ftdi_stopbits_type(stop_bits: &StopBits) -> ftdi_stopbits_type {
    match stop_bits {
        StopBits::StopBits1 => ftdi_stopbits_type::STOP_BIT_1,
        StopBits::StopBits15 => ftdi_stopbits_type::STOP_BIT_15,
        StopBits::StopBits2 => ftdi_stopbits_type::STOP_BIT_2,
        #[allow(unreachable_patterns)]
        _ => panic!("not supported StopBits: {:?}", stop_bits),
    }
}
fn make_ftdi_parity_type(parity: &Parity) -> ftdi_parity_type {
    match parity {
        Parity::None => ftdi_parity_type::NONE,
        Parity::Odd => ftdi_parity_type::ODD,
        Parity::Even => ftdi_parity_type::EVEN,
        Parity::Mark => ftdi_parity_type::MARK,
        Parity::Space => ftdi_parity_type::SPACE,
        #[allow(unreachable_patterns)]
        _ => panic!("not supported Parity: {:?}", parity),
    }
}
