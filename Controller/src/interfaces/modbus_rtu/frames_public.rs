// Based on https://modbus.org/docs/Modbus_Application_Protocol_V1_1b3.pdf section 6

use super::{
    frame::{Request, Response},
    helpers::{bits_byte_to_array, bits_bytes_to_slice_checked, bits_slice_to_bytes},
};
use anyhow::{anyhow, bail, ensure, Context, Error};
use std::{cmp::Ordering, iter};

// Generics for 0x01 and 0x02
#[derive(PartialEq, Eq, Debug)]
struct ReadBitsGenericRequest {
    starting_address: usize,
    number_of_bits: usize,
}
impl ReadBitsGenericRequest {
    pub fn new(
        starting_address: usize, // 1-based
        number_of_bits: usize,
    ) -> Result<Self, Error> {
        ensure!(
            (1..=65536).contains(&starting_address),
            "starting address must be between 1 and 65536"
        );
        ensure!(
            (1..=2000).contains(&number_of_bits),
            "number of bits must be between 1 and 2000"
        );
        ensure!(
            starting_address + number_of_bits - 1 <= 65536,
            "read operation will overflow address space"
        );
        Ok(Self {
            starting_address,
            number_of_bits,
        })
    }
    pub fn data(&self) -> Box<[u8]> {
        iter::empty()
            .chain(((self.starting_address - 1) as u16).to_be_bytes())
            .chain((self.number_of_bits as u16).to_be_bytes())
            .collect()
    }
}

#[derive(PartialEq, Eq, Debug)]
struct ReadBitsGenericResponse {
    bits_values: Box<[bool]>,
}
impl ReadBitsGenericResponse {
    pub fn new(bits_values: Box<[bool]>) -> Self {
        Self { bits_values }
    }

    pub fn bits_values(&self) -> &[bool] {
        &self.bits_values
    }
    pub fn into_bits_values(self) -> Box<[bool]> {
        self.bits_values
    }

    pub fn from_data(
        request: &ReadBitsGenericRequest,
        data: &[u8],
    ) -> Result<Option<Self>, anyhow::Error> {
        if data.is_empty() {
            return Ok(None);
        }
        let bit_bytes_count_received = data[0] as usize;

        let bit_bytes_count_expected = request.number_of_bits.div_ceil(8);
        ensure!(
            bit_bytes_count_received == bit_bytes_count_expected,
            "bit bytes count mismatch"
        );

        let bits_values_bytes = &data[1..];
        match bits_values_bytes.len().cmp(&bit_bytes_count_expected) {
            Ordering::Less => return Ok(None),
            Ordering::Equal => {}
            Ordering::Greater => return Err(anyhow!("bit bytes count overflow")),
        }

        let bits_values = bits_bytes_to_slice_checked(bits_values_bytes, request.number_of_bits)
            .context("bits_bytes_to_slice_checked")?;

        Ok(Some(Self { bits_values }))
    }
}

#[cfg(test)]
mod tests_read_bits_generic {
    use super::{ReadBitsGenericRequest, ReadBitsGenericResponse};

    #[test]
    fn request_response_1() {
        let request = ReadBitsGenericRequest::new(20, 19).unwrap();

        let request_data = request.data();

        let request_data_expected = [0x00, 0x13, 0x00, 0x13];

        assert_eq!(&*request_data, &request_data_expected);

        let response = ReadBitsGenericResponse::from_data(&request, &[0x03, 0xcd, 0x6b, 0x05])
            .unwrap()
            .unwrap();

        let response_expected = ReadBitsGenericResponse::new(
            vec![
                true, false, true, true, false, false, true, true, // 20-27
                true, true, false, true, false, true, true, false, // 28-35
                true, false, true, // 36-38
            ]
            .into_boxed_slice(),
        );

        assert_eq!(response, response_expected);
    }
}

// 0x01 - Read Coils
#[derive(PartialEq, Eq, Debug)]
pub struct ReadCoilsRequest {
    inner: ReadBitsGenericRequest,
}
impl ReadCoilsRequest {
    pub fn new(
        starting_address: usize,
        number_of_coils: usize,
    ) -> Result<Self, Error> {
        let inner =
            ReadBitsGenericRequest::new(starting_address, number_of_coils).context("inner")?;
        Ok(Self { inner })
    }
}
impl Request for ReadCoilsRequest {
    type Response = ReadCoilsResponse;

    fn function_code(&self) -> u8 {
        0x01
    }
    fn data(&self) -> Box<[u8]> {
        self.inner.data()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct ReadCoilsResponse {
    inner: ReadBitsGenericResponse,
}
impl ReadCoilsResponse {
    pub fn new(coils_values: Box<[bool]>) -> Self {
        let inner = ReadBitsGenericResponse::new(coils_values);
        Self { inner }
    }

    pub fn coils_values(&self) -> &[bool] {
        self.inner.bits_values()
    }
    pub fn into_coils_values(self) -> Box<[bool]> {
        self.inner.into_bits_values()
    }
}
impl Response for ReadCoilsResponse {
    type Request = ReadCoilsRequest;

    fn from_data(
        request: &Self::Request,
        data: &[u8],
    ) -> Result<Option<Self>, Error> {
        let inner =
            match ReadBitsGenericResponse::from_data(&request.inner, data).context("inner")? {
                Some(inner) => inner,
                None => return Ok(None),
            };
        Ok(Some(Self { inner }))
    }
}

// 0x02 - Read Discrete Inputs
#[derive(PartialEq, Eq, Debug)]
pub struct ReadDiscreteInputsRequest {
    inner: ReadBitsGenericRequest,
}
impl ReadDiscreteInputsRequest {
    pub fn new(
        starting_address: usize,
        number_of_discrete_inputs: usize,
    ) -> Result<Self, Error> {
        let inner = ReadBitsGenericRequest::new(starting_address, number_of_discrete_inputs)
            .context("inner")?;
        Ok(Self { inner })
    }
}
impl Request for ReadDiscreteInputsRequest {
    type Response = ReadDiscreteInputsResponse;

    fn function_code(&self) -> u8 {
        0x02
    }
    fn data(&self) -> Box<[u8]> {
        self.inner.data()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct ReadDiscreteInputsResponse {
    inner: ReadBitsGenericResponse,
}
impl ReadDiscreteInputsResponse {
    pub fn new(discrete_inputs_values: Box<[bool]>) -> Self {
        let inner = ReadBitsGenericResponse::new(discrete_inputs_values);
        Self { inner }
    }

    pub fn discrete_inputs_values(&self) -> &[bool] {
        self.inner.bits_values()
    }
    pub fn into_discrete_inputs_values(self) -> Box<[bool]> {
        self.inner.into_bits_values()
    }
}
impl Response for ReadDiscreteInputsResponse {
    type Request = ReadDiscreteInputsRequest;

    fn from_data(
        request: &Self::Request,
        data: &[u8],
    ) -> Result<Option<Self>, Error> {
        let inner =
            match ReadBitsGenericResponse::from_data(&request.inner, data).context("inner")? {
                Some(inner) => inner,
                None => return Ok(None),
            };
        Ok(Some(Self { inner }))
    }
}

// Generics for 0x03 and 0x04
#[derive(PartialEq, Eq, Debug)]
struct ReadWordsGenericRequest {
    starting_address: usize,
    number_of_words: usize,
}
impl ReadWordsGenericRequest {
    pub fn new(
        starting_address: usize,
        number_of_words: usize,
    ) -> Result<Self, Error> {
        ensure!(
            (1..=65536).contains(&starting_address),
            "starting address must be between 1 and 65536"
        );
        ensure!(
            (1..=125).contains(&number_of_words),
            "number of words must be between 1 and 125"
        );
        ensure!(
            starting_address + number_of_words - 1 <= 65536,
            "read operation will overflow address space"
        );
        Ok(Self {
            starting_address,
            number_of_words,
        })
    }
    pub fn data(&self) -> Box<[u8]> {
        iter::empty()
            .chain(((self.starting_address - 1) as u16).to_be_bytes())
            .chain((self.number_of_words as u16).to_be_bytes())
            .collect()
    }
}

#[derive(PartialEq, Eq, Debug)]
struct ReadWordsGenericResponse {
    words_values: Box<[u16]>,
}
impl ReadWordsGenericResponse {
    pub fn new(words_values: Box<[u16]>) -> Self {
        Self { words_values }
    }

    pub fn words_values(&self) -> &[u16] {
        self.words_values.as_ref()
    }
    pub fn into_words_values(self) -> Box<[u16]> {
        self.words_values
    }

    pub fn from_data(
        request: &ReadWordsGenericRequest,
        data: &[u8],
    ) -> Result<Option<Self>, anyhow::Error> {
        if data.is_empty() {
            return Ok(None);
        }
        let words_bytes_count_received = data[0] as usize;

        let words_bytes_count_expected = request.number_of_words * 2;
        ensure!(
            words_bytes_count_received == words_bytes_count_expected,
            "words bytes count mismatch"
        );

        let words_values_bytes = &data[1..];
        match words_values_bytes.len().cmp(&words_bytes_count_expected) {
            Ordering::Less => return Ok(None),
            Ordering::Equal => {}
            Ordering::Greater => return Err(anyhow!("word bytes count overflow")),
        }

        let words_values = words_values_bytes
            .array_chunks::<2>()
            .map(|words| u16::from_be_bytes(*words))
            .collect();

        Ok(Some(Self { words_values }))
    }
}

#[cfg(test)]
mod tests_read_words_generic {
    use super::{ReadWordsGenericRequest, ReadWordsGenericResponse};

    #[test]
    fn request_response_1() {
        let request = ReadWordsGenericRequest::new(108, 3).unwrap();

        let request_data = request.data();

        let request_data_expected = [0x00, 0x6b, 0x00, 0x03];

        assert_eq!(&*request_data, &request_data_expected);

        let response = ReadWordsGenericResponse::from_data(
            &request,
            &[0x06, 0x02, 0x2b, 0x00, 0x00, 0x00, 0x64],
        )
        .unwrap()
        .unwrap();

        let response_expected = ReadWordsGenericResponse::new(vec![555, 0, 100].into_boxed_slice());

        assert_eq!(response, response_expected);
    }
}

// 0x03 - Read Holding Registers
#[derive(PartialEq, Eq, Debug)]
pub struct ReadHoldingRegistersRequest {
    inner: ReadWordsGenericRequest,
}
impl ReadHoldingRegistersRequest {
    pub fn new(
        starting_address: usize,
        number_of_holding_registers: usize,
    ) -> Result<Self, Error> {
        let inner = ReadWordsGenericRequest::new(starting_address, number_of_holding_registers)
            .context("inner")?;
        Ok(Self { inner })
    }
}
impl Request for ReadHoldingRegistersRequest {
    type Response = ReadHoldingRegistersResponse;

    fn function_code(&self) -> u8 {
        0x03
    }
    fn data(&self) -> Box<[u8]> {
        self.inner.data()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct ReadHoldingRegistersResponse {
    inner: ReadWordsGenericResponse,
}
impl ReadHoldingRegistersResponse {
    pub fn new(holding_registers_values: Box<[u16]>) -> Self {
        let inner = ReadWordsGenericResponse::new(holding_registers_values);
        Self { inner }
    }

    pub fn holding_registers_values(&self) -> &[u16] {
        self.inner.words_values()
    }
    pub fn into_holding_registers_values(self) -> Box<[u16]> {
        self.inner.into_words_values()
    }
}
impl Response for ReadHoldingRegistersResponse {
    type Request = ReadHoldingRegistersRequest;

    fn from_data(
        request: &Self::Request,
        data: &[u8],
    ) -> Result<Option<Self>, Error> {
        let inner =
            match ReadWordsGenericResponse::from_data(&request.inner, data).context("inner")? {
                Some(inner) => inner,
                None => return Ok(None),
            };
        Ok(Some(Self { inner }))
    }
}

// 0x04 - Read Input Registers
#[derive(PartialEq, Eq, Debug)]
pub struct ReadInputRegistersRequest {
    inner: ReadWordsGenericRequest,
}
impl ReadInputRegistersRequest {
    pub fn new(
        starting_address: usize,
        number_of_input_registers: usize,
    ) -> Result<Self, Error> {
        let inner = ReadWordsGenericRequest::new(starting_address, number_of_input_registers)
            .context("inner")?;
        Ok(Self { inner })
    }
}
impl Request for ReadInputRegistersRequest {
    type Response = ReadInputRegistersResponse;

    fn function_code(&self) -> u8 {
        0x04
    }
    fn data(&self) -> Box<[u8]> {
        self.inner.data()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct ReadInputRegistersResponse {
    inner: ReadWordsGenericResponse,
}
impl ReadInputRegistersResponse {
    pub fn new(input_registers_values: Box<[u16]>) -> Self {
        let inner = ReadWordsGenericResponse::new(input_registers_values);
        Self { inner }
    }

    pub fn input_registers_values(&self) -> &[u16] {
        self.inner.words_values()
    }
    pub fn into_input_registers_values(self) -> Box<[u16]> {
        self.inner.into_words_values()
    }
}
impl Response for ReadInputRegistersResponse {
    type Request = ReadInputRegistersRequest;

    fn from_data(
        request: &Self::Request,
        data: &[u8],
    ) -> Result<Option<Self>, Error> {
        let inner =
            match ReadWordsGenericResponse::from_data(&request.inner, data).context("inner")? {
                Some(inner) => inner,
                None => return Ok(None),
            };
        Ok(Some(Self { inner }))
    }
}

// 0x05 - Write Single Coil
#[derive(PartialEq, Eq, Debug)]
pub struct WriteSingleCoilRequest {
    address: usize,
    value: bool,
}
impl WriteSingleCoilRequest {
    pub fn new(
        address: usize, // 1-based
        value: bool,
    ) -> Result<Self, Error> {
        ensure!(
            (1..=65536).contains(&address),
            "address must be between 1 and 65536"
        );
        Ok(Self { address, value })
    }
}
impl Request for WriteSingleCoilRequest {
    type Response = WriteSingleCoilResponse;

    fn function_code(&self) -> u8 {
        0x05
    }
    fn data(&self) -> Box<[u8]> {
        iter::empty()
            .chain(((self.address - 1) as u16).to_be_bytes())
            .chain(((if self.value { 0xFF00 } else { 0x0000 }) as u16).to_be_bytes())
            .collect()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct WriteSingleCoilResponse {}
impl WriteSingleCoilResponse {
    pub fn new() -> Self {
        Self {}
    }
}
impl Response for WriteSingleCoilResponse {
    type Request = WriteSingleCoilRequest;

    fn from_data(
        request: &Self::Request,
        data: &[u8],
    ) -> Result<Option<Self>, Error> {
        if data.len() < 2 {
            return Ok(None);
        }
        let address_received = u16::from_be_bytes(data[0..2].try_into().unwrap());
        let address_received: usize = (address_received as usize) + 1;
        ensure!(address_received == request.address, "address mismatch");

        if data.len() < 4 {
            return Ok(None);
        }
        let value_received: u16 = u16::from_be_bytes(data[2..4].try_into().unwrap());
        let value_received = match value_received {
            0xFF00 => true,
            0x0000 => false,
            _ => bail!("invalid value"),
        };
        ensure!(value_received == request.value, "value mismatch");

        ensure!(data.len() == 4, "data overflow");
        Ok(Some(Self {}))
    }
}

#[cfg(test)]
mod tests_write_single_coil {
    use super::{Request, Response, WriteSingleCoilRequest, WriteSingleCoilResponse};

    #[test]
    fn request_response_1() {
        let request = WriteSingleCoilRequest::new(173, true).unwrap();

        let request_data = request.data();

        let request_data_expected = [0x00, 0xac, 0xff, 0x00];

        assert_eq!(&*request_data, &request_data_expected);

        let response = WriteSingleCoilResponse::from_data(&request, &[0x00, 0xac, 0xff, 0x00])
            .unwrap()
            .unwrap();

        let response_expected = WriteSingleCoilResponse::new();

        assert_eq!(response, response_expected);
    }
}

// 0x06 - Write Single Register
#[derive(PartialEq, Eq, Debug)]
pub struct WriteSingleRegisterRequest {
    address: usize,
    value: u16,
}
impl WriteSingleRegisterRequest {
    pub fn new(
        address: usize, // 1-based
        value: u16,
    ) -> Result<Self, Error> {
        ensure!(
            (1..=65536).contains(&address),
            "address must be between 1 and 65536"
        );
        Ok(Self { address, value })
    }
}
impl Request for WriteSingleRegisterRequest {
    type Response = WriteSingleRegisterResponse;

    fn function_code(&self) -> u8 {
        0x06
    }
    fn data(&self) -> Box<[u8]> {
        iter::empty()
            .chain(((self.address - 1) as u16).to_be_bytes())
            .chain(self.value.to_be_bytes())
            .collect()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct WriteSingleRegisterResponse {}
impl WriteSingleRegisterResponse {
    pub fn new() -> Self {
        Self {}
    }
}
impl Response for WriteSingleRegisterResponse {
    type Request = WriteSingleRegisterRequest;

    fn from_data(
        request: &Self::Request,
        data: &[u8],
    ) -> Result<Option<Self>, Error> {
        if data.len() < 2 {
            return Ok(None);
        }
        let address_received = u16::from_be_bytes(data[0..2].try_into().unwrap());
        let address_received: usize = (address_received as usize) + 1;
        ensure!(address_received == request.address, "address mismatch");

        if data.len() < 4 {
            return Ok(None);
        }
        let value_received: u16 = u16::from_be_bytes(data[2..4].try_into().unwrap());
        ensure!(value_received == request.value, "value mismatch");

        ensure!(data.len() == 4, "data overflow");
        Ok(Some(Self {}))
    }
}

#[cfg(test)]
mod tests_write_single_register {
    use super::{Request, Response, WriteSingleRegisterRequest, WriteSingleRegisterResponse};

    #[test]
    fn request_response_1() {
        let request = WriteSingleRegisterRequest::new(2, 3).unwrap();

        let request_data = request.data();

        let request_data_expected = [0x00, 0x01, 0x00, 0x03];

        assert_eq!(&*request_data, &request_data_expected);

        let response = WriteSingleRegisterResponse::from_data(&request, &[0x00, 0x01, 0x00, 0x03])
            .unwrap()
            .unwrap();

        let response_expected = WriteSingleRegisterResponse::new();

        assert_eq!(response, response_expected);
    }
}

// 0x07 - Read Exception Status
#[derive(PartialEq, Eq, Debug)]
pub struct ReadExceptionStatusRequest {}
impl ReadExceptionStatusRequest {
    pub fn new() -> Self {
        Self {}
    }
}
impl Request for ReadExceptionStatusRequest {
    type Response = ReadExceptionStatusResponse;

    fn function_code(&self) -> u8 {
        0x07
    }
    fn data(&self) -> Box<[u8]> {
        Box::new([])
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct ReadExceptionStatusResponse {
    exception_status_outputs: [bool; 8],
}
impl ReadExceptionStatusResponse {
    pub fn new(exception_status_outputs: [bool; 8]) -> Self {
        Self {
            exception_status_outputs,
        }
    }
}
impl Response for ReadExceptionStatusResponse {
    type Request = ReadExceptionStatusRequest;

    fn from_data(
        _request: &Self::Request,
        data: &[u8],
    ) -> Result<Option<Self>, Error> {
        #[allow(clippy::len_zero)]
        if data.len() < 1 {
            return Ok(None);
        }

        let exception_status_outputs_byte = data[0];
        let exception_status_outputs = bits_byte_to_array(exception_status_outputs_byte);

        ensure!(data.len() == 1, "data overflow");

        Ok(Some(Self {
            exception_status_outputs,
        }))
    }
}

#[cfg(test)]
mod tests_read_exception_status {
    use super::{ReadExceptionStatusRequest, ReadExceptionStatusResponse, Request, Response};

    #[test]
    fn request_response_1() {
        let request = ReadExceptionStatusRequest::new();

        let request_data = request.data();

        assert!(request_data.is_empty());

        let response = ReadExceptionStatusResponse::from_data(&request, &[0x6d])
            .unwrap()
            .unwrap();

        let response_expected =
            ReadExceptionStatusResponse::new([true, false, true, true, false, true, true, false]);

        assert_eq!(response, response_expected);
    }
}

// 0x08 - Diagnostics
// TODO

// 0x0B - Get Comm Event Counter
// TODO

// 0x0C - Get Comm Event Log
// TODO

// 0x0F - Write Multiple Coils
#[derive(PartialEq, Eq, Debug)]
pub struct WriteMultipleCoilsRequest {
    starting_address: usize,
    values: Box<[bool]>,
}
impl WriteMultipleCoilsRequest {
    pub fn new(
        starting_address: usize,
        values: Box<[bool]>,
    ) -> Result<Self, Error> {
        ensure!(
            (1..=65536).contains(&starting_address),
            "starting address must be between 1 and 65536"
        );
        ensure!(
            (1..=1968).contains(&values.len()),
            "number of coils must be between 1 and 1968"
        );
        ensure!(
            starting_address + values.len() - 1 <= 65536,
            "write operation will overflow address space"
        );
        Ok(Self {
            starting_address,
            values,
        })
    }
}
impl Request for WriteMultipleCoilsRequest {
    type Response = WriteMultipleCoilsResponse;

    fn function_code(&self) -> u8 {
        0x0f
    }
    fn data(&self) -> Box<[u8]> {
        let values_bytes = bits_slice_to_bytes(&self.values);

        iter::empty()
            .chain(((self.starting_address - 1) as u16).to_be_bytes())
            .chain((self.values.len() as u16).to_be_bytes())
            .chain((values_bytes.len() as u8).to_be_bytes())
            .chain(values_bytes.iter().copied())
            .collect()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct WriteMultipleCoilsResponse {}
impl WriteMultipleCoilsResponse {
    pub fn new() -> Self {
        Self {}
    }
}
impl Response for WriteMultipleCoilsResponse {
    type Request = WriteMultipleCoilsRequest;

    fn from_data(
        request: &Self::Request,
        data: &[u8],
    ) -> Result<Option<Self>, Error> {
        if data.len() < 2 {
            return Ok(None);
        }
        let starting_address_received = u16::from_be_bytes(data[0..2].try_into().unwrap());
        let starting_address_received: usize = (starting_address_received as usize) + 1;
        ensure!(
            starting_address_received == request.starting_address,
            "starting address mismatch"
        );

        if data.len() < 4 {
            return Ok(None);
        }
        let number_of_coils_received: u16 = u16::from_be_bytes(data[2..4].try_into().unwrap());
        ensure!(
            number_of_coils_received == request.values.len() as u16,
            "number of coils mismatch"
        );

        ensure!(data.len() == 4, "data overflow");
        Ok(Some(Self {}))
    }
}

#[cfg(test)]
mod tests_write_multiple_coils {
    use super::{Request, Response, WriteMultipleCoilsRequest, WriteMultipleCoilsResponse};

    #[test]
    fn request_response_1() {
        let request = WriteMultipleCoilsRequest::new(
            20,
            vec![
                true, false, true, true, false, false, true, true, true, false,
            ]
            .into_boxed_slice(),
        )
        .unwrap();

        let request_data = request.data();

        let request_data_expected = [0x00, 0x13, 0x00, 0x0a, 0x02, 0xcd, 0x01];

        assert_eq!(&*request_data, &request_data_expected);

        let response = WriteMultipleCoilsResponse::from_data(&request, &[0x00, 0x13, 0x00, 0x0a])
            .unwrap()
            .unwrap();

        let response_expected = WriteMultipleCoilsResponse::new();

        assert_eq!(response, response_expected);
    }
}

// 0x10 - Write Multiple Registers
#[derive(PartialEq, Eq, Debug)]
pub struct WriteMultipleRegistersRequest {
    starting_address: usize,
    values: Box<[u16]>,
}
impl WriteMultipleRegistersRequest {
    pub fn new(
        starting_address: usize,
        values: Box<[u16]>,
    ) -> Result<Self, Error> {
        ensure!(
            (1..=65536).contains(&starting_address),
            "starting address must be between 1 and 65536"
        );
        ensure!(
            (1..=123).contains(&values.len()),
            "number of registers must be between 1 and 123"
        );
        ensure!(
            starting_address + values.len() - 1 <= 65536,
            "write operation will overflow address space"
        );
        Ok(Self {
            starting_address,
            values,
        })
    }
}
impl Request for WriteMultipleRegistersRequest {
    type Response = WriteMultipleRegistersResponse;

    fn function_code(&self) -> u8 {
        0x10
    }
    fn data(&self) -> Box<[u8]> {
        iter::empty()
            .chain(((self.starting_address - 1) as u16).to_be_bytes())
            .chain((self.values.len() as u16).to_be_bytes())
            .chain(((self.values.len() * 2) as u8).to_be_bytes())
            .chain(self.values.iter().flat_map(|value| value.to_be_bytes()))
            .collect()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct WriteMultipleRegistersResponse {}
impl WriteMultipleRegistersResponse {
    pub fn new() -> Self {
        Self {}
    }
}
impl Response for WriteMultipleRegistersResponse {
    type Request = WriteMultipleRegistersRequest;

    fn from_data(
        request: &Self::Request,
        data: &[u8],
    ) -> Result<Option<Self>, Error> {
        if data.len() < 2 {
            return Ok(None);
        }
        let starting_address_received = u16::from_be_bytes(data[0..2].try_into().unwrap());
        let starting_address_received: usize = (starting_address_received as usize) + 1;
        ensure!(
            starting_address_received == request.starting_address,
            "starting address mismatch"
        );

        if data.len() < 4 {
            return Ok(None);
        }
        let number_of_registers_received: u16 = u16::from_be_bytes(data[2..4].try_into().unwrap());
        ensure!(
            number_of_registers_received == request.values.len() as u16,
            "number of registers mismatch"
        );

        ensure!(data.len() == 4, "data overflow");
        Ok(Some(Self {}))
    }
}

#[cfg(test)]
mod tests_write_multiple_registers {
    use super::{Request, Response, WriteMultipleRegistersRequest, WriteMultipleRegistersResponse};

    #[test]
    fn request_response_1() {
        let request =
            WriteMultipleRegistersRequest::new(2, vec![0x00a, 0x0102].into_boxed_slice()).unwrap();

        let request_data = request.data();

        let request_data_expected = [0x00, 0x01, 0x00, 0x02, 0x04, 0x00, 0x0a, 0x01, 0x02];

        assert_eq!(&*request_data, &request_data_expected);

        let response =
            WriteMultipleRegistersResponse::from_data(&request, &[0x00, 0x01, 0x00, 0x02])
                .unwrap()
                .unwrap();

        let response_expected = WriteMultipleRegistersResponse::new();

        assert_eq!(response, response_expected);
    }
}

// 0x11 - Report Server ID
// TODO

// 0x14 - Read File Record
// TODO

// 0x15 - Write File Record
// TODO

// 0x16 - Mask Write Register
// TODO

// 0x17 - Read/Write Multiple Registers
// TODO

// 0x18 - Read FIFO Queue
// TODO

// 0x2B - Encapsulated Interface Transport
// TODO
