use anyhow::{bail, Error};
use crc_all::Crc;
use std::{convert::TryInto, fmt, ops::Deref, slice};

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub struct AddressDeviceType([u8; Self::LENGTH]);
impl AddressDeviceType {
    pub const LENGTH: usize = 4;
    pub fn new(device_type: [u8; Self::LENGTH]) -> Result<Self, Error> {
        if !device_type.iter().all(|item| item.is_ascii_digit()) {
            bail!("invalid characters in device_type");
        }
        Ok(Self(device_type))
    }
    pub fn new_from_ordinal(ordinal: usize) -> Result<Self, Error> {
        let device_type_string = format!("{:0>4}", ordinal);
        Ok(Self(device_type_string.as_bytes()[..].try_into()?))
    }
}
impl Deref for AddressDeviceType {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl fmt::Display for AddressDeviceType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        f.write_str(std::str::from_utf8(self).unwrap())
    }
}
#[cfg(test)]
mod test_address_device_type {
    use super::*;

    #[test]
    fn new_1() {
        let address = AddressDeviceType::new(*b"000A");
        assert!(address.is_err());
    }

    #[test]
    fn new_2() {
        let address = AddressDeviceType::new(*b"0001");
        assert!(address.is_ok());
    }
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub struct AddressSerial([u8; Self::LENGTH]);
impl AddressSerial {
    pub const LENGTH: usize = 8;
    pub fn new(serial: [u8; Self::LENGTH]) -> Result<Self, Error> {
        if !serial.iter().all(|item| item.is_ascii_digit()) {
            bail!("invalid characters in serial");
        }
        Ok(Self(serial))
    }
}
impl Deref for AddressSerial {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl fmt::Display for AddressSerial {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        f.write_str(std::str::from_utf8(self).unwrap())
    }
}
#[cfg(test)]
mod test_address_serial {
    use super::*;

    #[test]
    fn new_1() {
        let address = AddressSerial::new(*b"0000000A");
        assert!(address.is_err());
    }

    #[test]
    fn new_2() {
        let address = AddressSerial::new(*b"00000001");
        assert!(address.is_ok());
    }
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub struct Address {
    device_type: AddressDeviceType,
    serial: AddressSerial,
}
impl Address {
    pub fn new(
        device_type: AddressDeviceType,
        serial: AddressSerial,
    ) -> Self {
        Self {
            device_type,
            serial,
        }
    }

    pub fn device_type(&self) -> &AddressDeviceType {
        &self.device_type
    }
    pub fn serial(&self) -> &AddressSerial {
        &self.serial
    }
}
impl fmt::Display for Address {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "{} {}", self.device_type(), self.serial())
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Payload(Box<[u8]>);
impl Payload {
    pub fn new(data: Box<[u8]>) -> Result<Self, Error> {
        if !data.iter().all(|item| item.is_ascii_graphic()) {
            bail!("invalid characters in payload");
        }
        Ok(Self(data))
    }
}
impl Deref for Payload {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
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
mod test_payload {
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

pub struct Frame {}
impl Frame {
    pub const CHAR_BEGIN: u8 = b'\n';
    pub const CHAR_END: u8 = b'\r';

    const CRC_POLY: u16 = 0x8005;
    const CRC_WIDTH: usize = 16;
    const CRC_INITIAL: u16 = 0xFFFF;
    const CRC_FINAL_XOR: u16 = 0x0000;
    const CRC_REFLECT: bool = true;

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

        let mut crc16 = Crc::<u16>::new(
            Self::CRC_POLY,
            Self::CRC_WIDTH,
            Self::CRC_INITIAL,
            Self::CRC_FINAL_XOR,
            Self::CRC_REFLECT,
        );
        crc16.update(slice::from_ref(&char_direction));
        crc16.update(&address.device_type);
        crc16.update(&address.serial);
        crc16.update(payload);
        let crc16 = crc16.finish();
        let crc16 = hex::encode_upper(crc16.to_be_bytes());
        let crc16 = crc16.as_bytes();

        let frame = [
            slice::from_ref(&Self::CHAR_BEGIN),
            slice::from_ref(&char_direction),
            &address.device_type,
            &address.serial,
            &crc16,
            &payload,
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

        if frame.len() < FRAME_LENGTH_MIN {
            bail!("frame too short");
        }

        if frame[0] != Self::CHAR_BEGIN {
            bail!("invalid begin character");
        }

        if frame[1]
            != (if service_mode {
                Self::CHAR_DIRECTION_SERVICE_IN
            } else {
                Self::CHAR_DIRECTION_NORMAL_IN
            })
        {
            bail!("invalid service_mode character");
        }

        let crc16_received = &frame[2..2 + 4];
        if !crc16_received
            .iter()
            .all(|item| item.is_ascii_uppercase() || item.is_ascii_digit())
        {
            bail!("invalid character in crc16");
        }
        let crc16_received = hex::decode(crc16_received)?;
        let crc16_received = u16::from_be_bytes((&crc16_received[..]).try_into().unwrap());

        let payload = Payload::new(Box::from(&frame[2 + 4..frame.len() - 1]))?;

        if frame[frame.len() - 1] != Frame::CHAR_END {
            bail!("invalid end character");
        }

        let mut crc16_expected = Crc::<u16>::new(
            Self::CRC_POLY,
            Self::CRC_WIDTH,
            Self::CRC_INITIAL,
            Self::CRC_FINAL_XOR,
            Self::CRC_REFLECT,
        );
        crc16_expected.update(slice::from_ref(&frame[1]));
        crc16_expected.update(&address.device_type);
        crc16_expected.update(&address.serial);
        crc16_expected.update(&payload);
        let crc16_expected = crc16_expected.finish();

        if crc16_expected != crc16_received {
            bail!(
                "invalid CRC16, expected: {:04X}, received: {:04X}",
                crc16_expected,
                crc16_received,
            );
        }

        Ok(payload)
    }
}
#[cfg(test)]
mod test_frame {
    use super::*;

    #[test]
    fn out_build_1() {
        assert_eq!(
            Frame::out_build(
                false,
                &Address::new(
                    AddressDeviceType::new(*b"0001").unwrap(),
                    AddressSerial::new(*b"98765432").unwrap(),
                ),
                &Payload::new(Box::from(*b"ChujDupaKamieniKupa")).unwrap(),
            )
            .as_ref(),
            &b"\n>000198765432BF20ChujDupaKamieniKupa\r"[..]
        );
    }
    #[test]
    fn out_build_2() {
        assert_eq!(
            Frame::out_build(
                true,
                &Address::new(
                    AddressDeviceType::new(*b"0006").unwrap(),
                    AddressSerial::new(*b"90083461").unwrap(),
                ),
                &Payload::new(Box::from(*b"#")).unwrap(),
            )
            .as_ref(),
            &b"\n}000690083461A17F#\r"[..]
        )
    }
    #[test]
    fn in_parse_1() {
        let frame = Frame::in_parse(
            b"",
            false,
            &Address::new(
                AddressDeviceType::new(*b"0001").unwrap(),
                AddressSerial::new(*b"98765432").unwrap(),
            ),
        );
        assert!(frame.is_err());
    }
    #[test]
    fn in_parse_2() {
        assert_eq!(
            Frame::in_parse(
                b"\n<A721ChujDupaKamieniKupa\r",
                false,
                &Address::new(
                    AddressDeviceType::new(*b"0001").unwrap(),
                    AddressSerial::new(*b"98765432").unwrap(),
                ),
            )
            .unwrap(),
            Payload::new(Box::from(*b"ChujDupaKamieniKupa")).unwrap(),
        );
    }
}
