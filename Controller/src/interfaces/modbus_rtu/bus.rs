use super::frame::{Exception, Request, Response};
use crate::interfaces::serial::{self, ftdi};
use anyhow::{bail, ensure, Context, Error};
use crc::{Crc, CRC_16_MODBUS};
use crossbeam::channel;
use futures::channel::oneshot;
use std::{
    any::Any, convert::TryInto, fmt::Debug, mem::ManuallyDrop, slice, thread, time::Duration,
};

pub struct Bus {
    ftdi_device: ftdi::DeviceFailSafe,
}
impl Bus {
    const FTDI_DEVICE_CONFIGURATION: ftdi::DeviceConfiguration = ftdi::DeviceConfiguration {
        latency_timer_ms: 10,
    };
    const FTDI_RETRY_COUNT: usize = 3;
    const FTDI_RETRY_INTERVAL: Duration = Duration::from_secs(1);

    const CRC16_ALGORITHM: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);

    const RESPONSE_LENGTH_MAX: usize = 256;

    pub fn new(
        descriptor: ftdi::Descriptor,
        baud_rate: usize,
        parity: serial::Parity,
    ) -> Self {
        let serial_configuration = serial::Configuration {
            baud_rate,
            bits: serial::Bits::Bits8,
            stop_bits: if parity != serial::Parity::None {
                serial::StopBits::StopBits1
            } else {
                serial::StopBits::StopBits2
            },
            parity,
        };

        let ftdi_device = ftdi::DeviceFailSafe::new(
            descriptor,
            serial_configuration,
            Self::FTDI_DEVICE_CONFIGURATION,
            Self::FTDI_RETRY_COUNT,
            Self::FTDI_RETRY_INTERVAL,
        );

        Self { ftdi_device }
    }

    pub fn transaction<T: Request>(
        &mut self,
        address: u8,
        request: &T,
        timeout: Duration,
    ) -> Result<T::Response, Error> {
        self.phase_send(address, request).context("phase_send")?;

        let response = self
            .phase_receive(address, request, timeout)
            .context("phase_receive")?;

        Ok(response)
    }

    fn phase_send<T: Request>(
        &mut self,
        address: u8,
        request: &T,
    ) -> Result<(), Error> {
        let payload = Self::serialize(address, request).context("serialize")?;

        self.ftdi_device.write(&payload).context("write")?;

        Ok(())
    }
    fn phase_receive<T: Request>(
        &mut self,
        address: u8,
        request: &T,
        timeout: Duration,
    ) -> Result<T::Response, Error> {
        let mut payload_buffer = Vec::new();
        let mut timeout = timeout;

        let response = loop {
            if timeout <= Duration::ZERO {
                bail!("timeout expired");
            }

            let payload = self.ftdi_device.read().context("read")?;
            if payload.is_empty() {
                timeout = timeout.saturating_sub(Duration::from_millis(
                    Self::FTDI_DEVICE_CONFIGURATION.latency_timer_ms as u64,
                ));
                continue;
            }

            if payload_buffer.len() + payload.len() > Self::RESPONSE_LENGTH_MAX {
                bail!("payload size exceeded");
            }
            let mut frame = payload.into_vec();
            payload_buffer.append(&mut frame);

            if let Some(response) =
                Self::parse(address, request, &payload_buffer).context("parse")?
            {
                break response;
            }
        };

        Ok(response)
    }

    fn serialize<T: Request>(
        address: u8,
        request: &T,
    ) -> Result<Box<[u8]>, Error> {
        ensure!((1..=255).contains(&address), "address out of range");

        let function_code = request.function_code();
        ensure!(
            (1..=127).contains(&function_code),
            "function code out of range"
        );

        let data = request.data();

        let mut crc = Self::CRC16_ALGORITHM.digest();
        crc.update(slice::from_ref(&address));
        crc.update(slice::from_ref(&function_code));
        crc.update(&data);
        let crc = crc.finalize();
        let crc = crc.to_le_bytes(); // crc has opposite byte order

        let payload = [
            slice::from_ref(&address),
            slice::from_ref(&function_code),
            &data,
            &crc,
        ]
        .concat();

        let payload = payload.into_boxed_slice();

        Ok(payload)
    }
    fn parse<T: Request>(
        address: u8,
        request: &T,
        payload: &[u8],
    ) -> Result<Option<T::Response>, Error> {
        let function_code_expected = request.function_code();

        // check for address compatibility
        #[allow(clippy::len_zero)]
        let address_received = if payload.len() >= 1 {
            payload[0]
        } else {
            return Ok(None);
        };
        ensure!(address_received == address, "response address mismatch");

        // check for function code matching and detect error
        let function_code_exception_mask_received = if payload.len() >= 2 {
            payload[1]
        } else {
            return Ok(None);
        };
        ensure!(
            function_code_exception_mask_received & !0x80 == function_code_expected,
            "function code mismatch"
        );
        let exception = function_code_exception_mask_received & 0x80 != 0x00;

        // extract payload
        // 4 is for address, function code, 0 bytes of data and crc
        let data = if payload.len() >= 4 {
            &payload[2..payload.len() - 2]
        } else {
            return Ok(None);
        };

        // extract real crc
        let crc_received = u16::from_le_bytes(
            (&payload[payload.len() - 2..payload.len()])
                .try_into()
                .unwrap(),
        );

        // try parsing the input
        // this allows to fetch more data before checking crc
        let response_or_exception = if !exception {
            let response = match T::Response::from_data(request, data).context("from_data")? {
                Some(response) => response,
                None => return Ok(None),
            };
            Result::<T::Response, Exception>::Ok(response)
        } else {
            let exception = match Exception::from_data(data).context("from_data")? {
                Some(exception) => exception,
                None => return Ok(None),
            };
            Result::<T::Response, Exception>::Err(exception)
        };

        // calculate crc of actual value
        let mut crc = Self::CRC16_ALGORITHM.digest();
        crc.update(slice::from_ref(&address));
        crc.update(slice::from_ref(&function_code_exception_mask_received));
        crc.update(data);
        let crc = crc.finalize();

        // final crc validations
        ensure!(crc == crc_received, "crc mismatch");

        // bail if it was exception
        let response = response_or_exception.context("response_or_exception")?;

        // all done :)
        Ok(Some(response))
    }
}

#[derive(Debug)]
pub struct AsyncBus {
    descriptor: ftdi::Descriptor,
    baud_rate: usize,
    parity: serial::Parity,

    transaction_sender: ManuallyDrop<channel::Sender<AsyncBusTransaction>>,
    worker_thread: ManuallyDrop<thread::JoinHandle<()>>,
}
impl AsyncBus {
    pub fn new(
        descriptor: ftdi::Descriptor,
        baud_rate: usize,
        parity: serial::Parity,
    ) -> Self {
        let (transaction_sender, transaction_receiver) = channel::unbounded();

        let worker_descriptor = descriptor.clone();
        let worker_thread = thread::Builder::new()
            .name(format!(
                "{}/modbus_rtu",
                descriptor.serial_number.to_str().unwrap()
            ))
            .spawn(move || {
                Self::thread_main(worker_descriptor, baud_rate, parity, transaction_receiver);
            })
            .unwrap();

        Self {
            descriptor,
            baud_rate,
            parity,

            transaction_sender: ManuallyDrop::new(transaction_sender),
            worker_thread: ManuallyDrop::new(worker_thread),
        }
    }

    pub async fn transaction<T: Request>(
        &self,
        address: u8,
        request: T,
        timeout: Duration,
    ) -> Result<T::Response, Error> {
        let request = RequestGenericWrapper::from_original(request);

        let (result_sender, result_receiver) = oneshot::channel();

        let transaction = AsyncBusTransaction {
            address,
            request,
            timeout,
            result_sender,
        };
        self.transaction_sender.send(transaction).unwrap();

        let response = result_receiver.await.unwrap()?;
        let response = response.into_original::<T::Response>();
        Ok(response)
    }

    fn thread_main(
        descriptor: ftdi::Descriptor,
        baud_rate: usize,
        parity: serial::Parity,

        transaction_receiver: channel::Receiver<AsyncBusTransaction>,
    ) {
        let mut bus = Bus::new(descriptor, baud_rate, parity);

        for transaction in transaction_receiver.iter() {
            let AsyncBusTransaction {
                address,
                request,
                timeout,
                result_sender,
            } = transaction;

            let result = bus.transaction(address, &request, timeout);

            let _ = result_sender.send(result);
        }
    }
}
impl Drop for AsyncBus {
    fn drop(&mut self) {
        // This ends the iteration
        unsafe { ManuallyDrop::drop(&mut self.transaction_sender) };

        // This joins and awaits the thread
        unsafe { ManuallyDrop::take(&mut self.worker_thread) }
            .join()
            .unwrap();
    }
}

struct AsyncBusTransaction {
    address: u8,
    request: RequestGenericWrapper,
    timeout: Duration,
    result_sender: oneshot::Sender<Result<ResponseGenericWrapper, Error>>,
}

trait RequestGeneric: Debug + Send + 'static {
    fn function_code(&self) -> u8;
    fn data(&self) -> Box<[u8]>;

    fn response_from_data(
        &self,
        request: &RequestGenericWrapper,
        data: &[u8],
    ) -> Result<Option<ResponseGenericWrapper>, Error>;

    fn as_any(&self) -> &dyn Any;
}
impl<T: Request> RequestGeneric for T {
    fn function_code(&self) -> u8 {
        self.function_code()
    }
    fn data(&self) -> Box<[u8]> {
        self.data()
    }

    fn response_from_data(
        &self,
        request: &RequestGenericWrapper,
        data: &[u8],
    ) -> Result<Option<ResponseGenericWrapper>, Error> {
        let request = request.as_original::<T>();
        let response = T::Response::from_data(request, data);
        let response = response.map(|response| response.map(ResponseGenericWrapper::from_original));
        response
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug)]
struct RequestGenericWrapper(Box<dyn RequestGeneric>);
impl RequestGenericWrapper {
    fn from_original<T: Request>(request: T) -> Self {
        Self(Box::new(request))
    }
    fn as_original<T: Request>(&self) -> &T {
        self.0.as_any().downcast_ref::<T>().unwrap()
    }
}
impl Request for RequestGenericWrapper {
    type Response = ResponseGenericWrapper;

    fn function_code(&self) -> u8 {
        self.0.function_code()
    }
    fn data(&self) -> Box<[u8]> {
        self.0.data()
    }
}

trait ResponseGeneric: Debug + Send + 'static {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}
impl<T: Response> ResponseGeneric for T {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

#[derive(Debug)]
struct ResponseGenericWrapper(Box<dyn ResponseGeneric>);
impl ResponseGenericWrapper {
    fn from_original<T: Response>(response: T) -> Self {
        Self(Box::new(response))
    }
    fn into_original<T: Response>(self) -> T {
        *self.0.into_any().downcast::<T>().unwrap()
    }
}
impl Response for ResponseGenericWrapper {
    type Request = RequestGenericWrapper;

    fn from_data(
        request: &Self::Request,
        data: &[u8],
    ) -> Result<Option<Self>, Error> {
        request.0.response_from_data(request, data)
    }
}

#[cfg(test)]
mod tests_bus {
    // https://ipc2u.com/articles/knowledge-base/modbus-rtu-made-simple-with-detailed-descriptions-and-examples/#read_discr_out

    use super::{
        super::frames_public::{ReadCoilsRequest, ReadCoilsResponse},
        Bus,
    };

    #[test]
    fn test_serialize_1() {
        let request = ReadCoilsRequest::new(20, 37).unwrap();
        assert_eq!(
            Bus::serialize(0x11, &request).unwrap().into_vec(),
            vec![0x11, 0x01, 0x00, 0x13, 0x00, 0x25, 0x0e, 0x84]
        );
    }

    #[test]
    fn test_parse_ok() {
        let request = ReadCoilsRequest::new(20, 37).unwrap();
        let response = ReadCoilsResponse::new(
            vec![
                true, false, true, true, false, false, true, true, // 20-27
                true, true, false, true, false, true, true, false, // 28 - 35
                false, true, false, false, true, true, false, true, // 36 - 43
                false, true, true, true, false, false, false, false, // 44 - 51
                true, true, false, true, true, // 52 - 56
            ]
            .into_boxed_slice(),
        );

        assert_eq!(
            Bus::parse(
                0x11,
                &request,
                &[0x11, 0x01, 0x05, 0xcd, 0x6b, 0xb2, 0x0e, 0x1b, 0x45, 0xe6]
            )
            .unwrap()
            .unwrap(),
            response
        );
    }
    #[test]
    fn test_parse_too_short() {
        let request = ReadCoilsRequest::new(20, 37).unwrap();

        assert!(Bus::parse(
            0x11,
            &request,
            &[0x11, 0x01, 0x05, 0xcd, 0x6b, 0xb2, 0x0e, 0x1b, 0x45]
        )
        .unwrap()
        .is_none());
    }
    #[test]
    fn test_parse_too_long() {
        let request = ReadCoilsRequest::new(20, 37).unwrap();

        assert!(Bus::parse(
            0x11,
            &request,
            &[0x11, 0x01, 0x05, 0xcd, 0x6b, 0xb2, 0x0e, 0x1b, 0x45, 0xe6, 0x00]
        )
        .is_err());
    }
    #[test]
    fn test_parse_empty() {
        let request = ReadCoilsRequest::new(20, 37).unwrap();

        assert!(Bus::parse(0x11, &request, &[]).unwrap().is_none());
    }
}

#[cfg(test)]
mod tests_generic_wrappers {
    use super::{
        super::frames_public::{ReadCoilsRequest, ReadCoilsResponse},
        Bus, RequestGenericWrapper, ResponseGenericWrapper,
    };

    #[test]
    fn test_request_response() {
        let request = ReadCoilsRequest::new(20, 37).unwrap();
        let request_generic = RequestGenericWrapper::from_original(request);

        assert_eq!(
            Bus::serialize(0x11, &request_generic).unwrap().into_vec(),
            vec![0x11, 0x01, 0x00, 0x13, 0x00, 0x25, 0x0e, 0x84]
        );

        let response = ReadCoilsResponse::new(
            vec![
                true, false, true, true, false, false, true, true, // 20-27
                true, true, false, true, false, true, true, false, // 28 - 35
                false, true, false, false, true, true, false, true, // 36 - 43
                false, true, true, true, false, false, false, false, // 44 - 51
                true, true, false, true, true, // 52 - 56
            ]
            .into_boxed_slice(),
        );

        let response_generic: ResponseGenericWrapper = Bus::parse(
            0x11,
            &request_generic,
            &[0x11, 0x01, 0x05, 0xcd, 0x6b, 0xb2, 0x0e, 0x1b, 0x45, 0xe6],
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            response_generic.into_original::<ReadCoilsResponse>(),
            response
        );
    }
}