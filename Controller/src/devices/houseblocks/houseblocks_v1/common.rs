use anyhow::{ensure, Context, Error};
use crc::{Crc, CRC_16_MODBUS};
use std::{fmt, slice, str};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AddressDeviceType([u8; Self::LENGTH]);
impl AddressDeviceType {
    pub const LENGTH: usize = 4;

    pub fn new(device_type: [u8; Self::LENGTH]) -> Result<Self, Error> {
        ensure!(
            device_type.iter().all(|item| item.is_ascii_digit()),
            "invalid characters"
        );
        ensure!(
            !device_type.iter().all(|item| *item == b'0'),
            "cannot be all zeros",
        );
        Ok(Self(device_type))
    }
    pub fn new_from_string(string: &str) -> Result<Self, Error> {
        ensure!(
            string.len() == Self::LENGTH,
            "expected length of {}",
            Self::LENGTH
        );
        let self_ = Self::new(string.as_bytes().try_into().unwrap()).context("new")?;
        Ok(self_)
    }
    pub fn new_from_ordinal(ordinal: usize) -> Result<Self, Error> {
        ensure!(
            (1..=9999).contains(&ordinal),
            "ordinal must be in range 0001-9999"
        );
        let device_type_string = format!("{:0>4}", ordinal);
        let self_ = Self::new_from_string(&device_type_string).context("new_from_string")?;
        Ok(self_)
    }

    pub fn as_bytes(&self) -> &[u8; Self::LENGTH] {
        &self.0
    }
}
impl str::FromStr for AddressDeviceType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new_from_string(s)
    }
}
impl fmt::Display for AddressDeviceType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        f.write_str(str::from_utf8(self.as_bytes()).unwrap())
    }
}
#[cfg(test)]
mod tests_address_device_type {
    use super::*;

    #[test]
    fn invalid_1() {
        AddressDeviceType::new(*b"000A").unwrap_err();
    }
    #[test]
    fn invalid_2() {
        AddressDeviceType::new(*b"0000").unwrap_err();
    }
    #[test]
    fn invalid_3() {
        AddressDeviceType::new_from_ordinal(0).unwrap_err();
    }
    #[test]
    fn invalid_4() {
        AddressDeviceType::new_from_ordinal(10000).unwrap_err();
    }

    #[test]
    fn valid_1() {
        AddressDeviceType::new(*b"0001").unwrap();
    }
    #[test]
    fn valid_2() {
        let address = AddressDeviceType::new(*b"9999").unwrap();
        assert_eq!(*address.as_bytes(), [b'9', b'9', b'9', b'9']);
    }
    #[test]
    fn valid_3() {
        let address = AddressDeviceType::new(*b"1234").unwrap();
        assert_eq!(*address.as_bytes(), [b'1', b'2', b'3', b'4']);
    }
    #[test]
    fn valid_4() {
        let address = AddressDeviceType::new_from_ordinal(1).unwrap();
        assert_eq!(*address.as_bytes(), [b'0', b'0', b'0', b'1']);
    }
    #[test]
    fn valid_5() {
        let address = AddressDeviceType::new_from_ordinal(9998).unwrap();
        assert_eq!(*address.as_bytes(), [b'9', b'9', b'9', b'8']);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AddressSerial([u8; Self::LENGTH]);
impl AddressSerial {
    pub const LENGTH: usize = 8;

    pub fn new(serial: [u8; Self::LENGTH]) -> Result<Self, Error> {
        ensure!(
            serial.iter().all(|item| item.is_ascii_digit()),
            "invalid characters"
        );
        ensure!(
            !serial.iter().all(|item| *item == b'0'),
            "cannot be all zeros",
        );
        Ok(Self(serial))
    }
    pub fn new_from_string(string: &str) -> Result<Self, Error> {
        ensure!(
            string.len() == Self::LENGTH,
            "expected length of {}",
            Self::LENGTH
        );
        let self_ = Self::new(string.as_bytes().try_into().unwrap()).context("new")?;
        Ok(self_)
    }
    pub fn new_from_ordinal(ordinal: usize) -> Result<Self, Error> {
        ensure!(
            (1..=9999_9999).contains(&ordinal),
            "ordinal must be in range 00000001-99999999"
        );
        let serial = format!("{:0>8}", ordinal);
        let self_ = Self::new_from_string(&serial).context("new_from_string")?;
        Ok(self_)
    }

    pub fn as_bytes(&self) -> &[u8; Self::LENGTH] {
        &self.0
    }
}
impl str::FromStr for AddressSerial {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new_from_string(s)
    }
}
impl fmt::Display for AddressSerial {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        f.write_str(str::from_utf8(self.as_bytes()).unwrap())
    }
}
#[cfg(test)]
mod tests_address_serial {
    use super::*;

    #[test]
    fn invalid_1() {
        AddressSerial::new(*b"0000000A").unwrap_err();
    }
    #[test]
    fn invalid_2() {
        AddressSerial::new(*b"00000000").unwrap_err();
    }
    #[test]
    fn invalid_3() {
        AddressSerial::new_from_ordinal(0).unwrap_err();
    }
    #[test]
    fn invalid_4() {
        AddressSerial::new_from_ordinal(1_0000_0000).unwrap_err();
    }

    #[test]
    fn valid_1() {
        AddressSerial::new(*b"00000001").unwrap();
    }
    #[test]
    fn valid_2() {
        let address = AddressSerial::new(*b"99999999").unwrap();
        assert_eq!(
            *address.as_bytes(),
            [b'9', b'9', b'9', b'9', b'9', b'9', b'9', b'9']
        );
    }
    #[test]
    fn valid_3() {
        let address = AddressSerial::new(*b"12345678").unwrap();
        assert_eq!(
            *address.as_bytes(),
            [b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8']
        );
    }
    #[test]
    fn valid_4() {
        let address = AddressSerial::new_from_ordinal(1).unwrap();
        assert_eq!(
            *address.as_bytes(),
            [b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'1']
        );
    }
    #[test]
    fn valid_5() {
        let address = AddressSerial::new_from_ordinal(99999998).unwrap();
        assert_eq!(
            *address.as_bytes(),
            [b'9', b'9', b'9', b'9', b'9', b'9', b'9', b'8']
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Address {
    pub device_type: AddressDeviceType,
    pub serial: AddressSerial,
}
impl fmt::Display for Address {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "{} {}", self.device_type, self.serial)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Payload(Box<[u8]>);
impl Payload {
    pub fn new(data: Box<[u8]>) -> Result<Self, Error> {
        ensure!(
            data.iter().all(|item| item.is_ascii_graphic()),
            "invalid characters in payload"
        );
        Ok(Self(data))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
impl fmt::Display for Payload {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "{:X?}", self.0.as_ref())
    }
}
#[cfg(test)]
mod tests_payload {
    use super::*;

    #[test]
    fn new_1() {
        let payload = Payload::new(Box::from(*b"aaa\n"));
        assert!(payload.is_err());
    }
    #[test]
    fn new_2() {
        let payload = Payload::new(Box::from(*b"aA09"));
        assert!(payload.is_ok());
    }
}

#[derive(Debug)]
pub struct Frame {}
impl Frame {
    pub const CHAR_BEGIN: u8 = b'\n';
    pub const CHAR_END: u8 = b'\r';

    const CRC_HASHER: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);

    const CHAR_DIRECTION_NORMAL_IN: u8 = b'<';
    const CHAR_DIRECTION_NORMAL_OUT: u8 = b'>';
    const CHAR_DIRECTION_SERVICE_IN: u8 = b'{';
    const CHAR_DIRECTION_SERVICE_OUT: u8 = b'}';

    pub fn out_build(
        service_mode: bool,
        address: &Address,
        payload: &Payload,
    ) -> Box<[u8]> {
        let char_direction = if service_mode {
            &Self::CHAR_DIRECTION_SERVICE_OUT
        } else {
            &Self::CHAR_DIRECTION_NORMAL_OUT
        };

        let mut crc16 = Self::CRC_HASHER.digest();
        crc16.update(slice::from_ref(char_direction));
        crc16.update(address.device_type.as_bytes());
        crc16.update(address.serial.as_bytes());
        crc16.update(payload.as_bytes());
        let crc16 = crc16.finalize();
        let crc16 = hex::encode_upper(crc16.to_be_bytes());
        let crc16 = crc16.as_bytes();

        let frame = [
            slice::from_ref(&Self::CHAR_BEGIN),
            slice::from_ref(char_direction),
            address.device_type.as_bytes(),
            address.serial.as_bytes(),
            crc16,
            payload.as_bytes(),
            slice::from_ref(&Self::CHAR_END),
        ]
        .concat();

        Box::from(frame)
    }

    pub fn in_parse(
        frame: &[u8],

        service_mode: bool,
        address: &Address,
    ) -> Result<Payload, Error> {
        pub const FRAME_LENGTH_MIN: usize = 1 + 1 + 4 /* + 0 */ + 1;

        ensure!(frame.len() >= FRAME_LENGTH_MIN, "frame too short");

        ensure!(frame[0] == Self::CHAR_BEGIN, "invalid begin character");

        ensure!(
            frame[1]
                == (if service_mode {
                    Self::CHAR_DIRECTION_SERVICE_IN
                } else {
                    Self::CHAR_DIRECTION_NORMAL_IN
                }),
            "invalid service_mode character"
        );

        let crc16_received = &frame[2..2 + 4];
        ensure!(
            crc16_received
                .iter()
                .all(|item| item.is_ascii_uppercase() || item.is_ascii_digit()),
            "invalid character in crc16"
        );
        let crc16_received = hex::decode(crc16_received).context("decode")?;
        let crc16_received = u16::from_be_bytes(crc16_received[..].try_into().unwrap());

        let payload = Payload::new(Box::from(&frame[2 + 4..frame.len() - 1])).context("new")?;

        ensure!(
            frame[frame.len() - 1] == Frame::CHAR_END,
            "invalid end character"
        );

        let mut crc16_expected = Self::CRC_HASHER.digest();
        crc16_expected.update(slice::from_ref(&frame[1]));
        crc16_expected.update(address.device_type.as_bytes());
        crc16_expected.update(address.serial.as_bytes());
        crc16_expected.update(payload.as_bytes());
        let crc16_expected = crc16_expected.finalize();

        ensure!(
            crc16_expected == crc16_received,
            "invalid CRC16, expected: {:04X}, received: {:04X}",
            crc16_expected,
            crc16_received,
        );

        Ok(payload)
    }
}
#[cfg(test)]
mod tests_frame {
    use super::*;

    #[test]
    fn out_build_1() {
        let frame = Frame::out_build(
            false,
            &Address {
                device_type: AddressDeviceType::new(*b"0001").unwrap(),
                serial: AddressSerial::new(*b"98765432").unwrap(),
            },
            &Payload::new(Box::from(*b"ChujDupaKamieniKupa")).unwrap(),
        );

        let frame_expected = b"\n>000198765432BF20ChujDupaKamieniKupa\r";

        assert_eq!(frame.as_ref(), &frame_expected[..]);
    }
    #[test]
    fn out_build_2() {
        let frame = Frame::out_build(
            true,
            &Address {
                device_type: AddressDeviceType::new(*b"0006").unwrap(),
                serial: AddressSerial::new(*b"90083461").unwrap(),
            },
            &Payload::new(Box::from(*b"#")).unwrap(),
        );

        let frame_expected = b"\n}000690083461A17F#\r";

        assert_eq!(frame.as_ref(), &frame_expected[..]);
    }

    #[test]
    fn in_parse_1() {
        let payload = Frame::in_parse(
            b"",
            false,
            &Address {
                device_type: AddressDeviceType::new(*b"0001").unwrap(),
                serial: AddressSerial::new(*b"98765432").unwrap(),
            },
        );

        assert!(payload.is_err());
    }
    #[test]
    fn in_parse_2() {
        let payload = Frame::in_parse(
            b"\n<A721ChujDupaKamieniKupa\r",
            false,
            &Address {
                device_type: AddressDeviceType::new(*b"0001").unwrap(),
                serial: AddressSerial::new(*b"98765432").unwrap(),
            },
        )
        .unwrap();

        let payload_expected = Payload::new(Box::from(*b"ChujDupaKamieniKupa")).unwrap();

        assert_eq!(payload, payload_expected,);
    }
}
