use super::super::super::houseblocks_v1::common::Payload;
use anyhow::{bail, ensure, Context, Error};
use arrayvec::ArrayVec;
use std::slice;

pub struct Parser<'a> {
    iterator: slice::Iter<'a, u8>,
}
impl<'a> Parser<'a> {
    pub fn from_payload(payload: &'a Payload) -> Self {
        Self {
            iterator: payload.as_bytes().iter(),
        }
    }

    pub fn get_byte(&mut self) -> Option<u8> {
        self.iterator.next().copied()
    }
    pub fn expect_byte(&mut self) -> Result<u8, Error> {
        let byte = match self.get_byte() {
            Some(byte) => byte,
            None => bail!("premature data end"),
        };
        Ok(byte)
    }
    pub fn expect_bool(&mut self) -> Result<bool, Error> {
        let value = match self.expect_byte().context("expect_byte")? {
            b'0' => false,
            b'1' => true,
            value => bail!("invalid character for bool: {}", value),
        };
        Ok(value)
    }
    pub fn expect_u8(&mut self) -> Result<u8, Error> {
        let value_hex = [
            self.expect_byte().context("expect_byte 1")?,
            self.expect_byte().context("expect_byte 2")?,
        ];
        let mut value = [0u8; 1];
        hex::decode_to_slice(value_hex, &mut value).context("decode_to_slice")?;
        Ok(u8::from_be_bytes(value))
    }
    pub fn expect_u16(&mut self) -> Result<u16, Error> {
        let value_hex = [
            self.expect_byte().context("expect_byte 1")?,
            self.expect_byte().context("expect_byte 2")?,
            self.expect_byte().context("expect_byte 3")?,
            self.expect_byte().context("expect_byte 4")?,
        ];
        let mut value = [0u8; 2];
        hex::decode_to_slice(value_hex, &mut value).context("decode_to_slice")?;
        Ok(u16::from_be_bytes(value))
    }
    pub fn expect_bool_array_8(&mut self) -> Result<[bool; 8], Error> {
        let bits = self.expect_u8().context("expect_u8")?;
        let value = (0..8)
            .map(|index| (bits & (1 << index)) != 0)
            .collect::<ArrayVec<bool, 8>>()
            .into_inner()
            .unwrap();
        Ok(value)
    }
    pub fn expect_bool_array_16(&mut self) -> Result<[bool; 16], Error> {
        let bits = self.expect_u16().context("expect_u16")?;
        let value = (0..16)
            .map(|index| (bits & (1 << index)) != 0)
            .collect::<ArrayVec<bool, 16>>()
            .into_inner()
            .unwrap();
        Ok(value)
    }
    pub fn expect_end(&self) -> Result<(), Error> {
        ensure!(self.iterator.is_empty(), "more data available");
        Ok(())
    }
}
#[cfg(test)]
mod tests_parser {
    use super::{super::super::super::houseblocks_v1::common::Payload, Parser};

    #[test]
    fn get_byte_1() {
        let payload = Payload::new(Box::from(*b"")).unwrap();
        let mut parser = Parser::from_payload(&payload);
        assert_eq!(parser.get_byte(), None);
    }
    #[test]
    fn get_byte_2() {
        let payload = Payload::new(Box::from(*b"AZ")).unwrap();
        let mut parser = Parser::from_payload(&payload);
        assert_eq!(parser.get_byte(), Some(b'A'));
        assert_eq!(parser.get_byte(), Some(b'Z'));
        assert_eq!(parser.get_byte(), None);
        assert_eq!(parser.get_byte(), None);
    }

    #[test]
    fn expect_end_1() {
        let payload = Payload::new(Box::from(*b"")).unwrap();
        let mut parser = Parser::from_payload(&payload);
        assert_eq!(parser.get_byte(), None);
        assert!(parser.expect_end().is_ok());
    }
    #[test]
    fn expect_end_2() {
        let payload = Payload::new(Box::from(*b"AZ")).unwrap();
        let mut parser = Parser::from_payload(&payload);
        assert!(parser.expect_end().is_err());
        assert_eq!(parser.get_byte(), Some(b'A'));
        assert_eq!(parser.get_byte(), Some(b'Z'));
        assert!(parser.expect_end().is_ok());
    }

    #[test]
    fn expect_byte_1() {
        let payload = Payload::new(Box::from(*b"")).unwrap();
        let mut parser = Parser::from_payload(&payload);
        assert!(parser.expect_byte().is_err());
    }
    #[test]
    fn expect_byte_2() {
        let payload = Payload::new(Box::from(*b"AZ")).unwrap();
        let mut parser = Parser::from_payload(&payload);
        assert_eq!(parser.expect_byte().unwrap(), b'A');
        assert_eq!(parser.expect_byte().unwrap(), b'Z');
        assert!(parser.expect_byte().is_err());
        assert!(parser.expect_byte().is_err());
    }

    #[test]
    fn expect_u8_1() {
        let payload = Payload::new(Box::from(*b"GG")).unwrap();
        let mut parser = Parser::from_payload(&payload);
        assert!(parser.expect_u8().is_err());
    }
    #[test]
    fn expect_u8_2() {
        let payload = Payload::new(Box::from(*b"0")).unwrap();
        let mut parser = Parser::from_payload(&payload);
        assert!(parser.expect_u8().is_err());
    }
    #[test]
    fn expect_u8_3() {
        let payload = Payload::new(Box::from(*b"00FF")).unwrap();
        let mut parser = Parser::from_payload(&payload);
        assert_eq!(parser.expect_u8().unwrap(), 0);
        assert_eq!(parser.expect_u8().unwrap(), 255);
        assert!(parser.expect_u8().is_err());
    }
    #[test]
    fn expect_u8_4() {
        let payload = Payload::new(Box::from(*b"00FFAA123445EE")).unwrap();
        let mut parser = Parser::from_payload(&payload);
        assert_eq!(parser.expect_u8().unwrap(), 0);
        assert_eq!(parser.expect_u8().unwrap(), 255);
        assert_eq!(parser.expect_u8().unwrap(), 0xAA);
        assert_eq!(parser.expect_u8().unwrap(), 0x12);
        assert_eq!(parser.expect_u8().unwrap(), 0x34);
        assert_eq!(parser.expect_u8().unwrap(), 0x45);
        assert_eq!(parser.expect_u8().unwrap(), 0xEE);
        assert!(parser.expect_u8().is_err());
    }

    #[test]
    fn expect_u16_1() {
        let payload = Payload::new(Box::from(*b"0000FFFF1234EDCB")).unwrap();
        let mut parser = Parser::from_payload(&payload);
        assert_eq!(parser.expect_u16().unwrap(), 0x0000);
        assert_eq!(parser.expect_u16().unwrap(), 0xFFFF);
        assert_eq!(parser.expect_u16().unwrap(), 0x1234);
        assert_eq!(parser.expect_u16().unwrap(), 0xEDCB);
        assert!(parser.expect_u16().is_err());
    }

    #[test]
    fn expect_bool_array_8_1() {
        let payload = Payload::new(Box::from(*b"A3")).unwrap();
        let mut parser = Parser::from_payload(&payload);
        assert_eq!(
            parser.expect_bool_array_8().unwrap(),
            [true, true, false, false, false, true, false, true]
        );
        assert!(parser.expect_end().is_ok());
    }

    #[test]
    fn expect_bool_array_16_1() {
        let payload = Payload::new(Box::from(*b"80C2")).unwrap();
        let mut parser = Parser::from_payload(&payload);
        assert_eq!(
            parser.expect_bool_array_16().unwrap(),
            [
                false, true, false, false, false, false, true, true, // break
                false, false, false, false, false, false, false, true, // break
            ]
        );
        assert!(parser.expect_end().is_ok());
    }
}
