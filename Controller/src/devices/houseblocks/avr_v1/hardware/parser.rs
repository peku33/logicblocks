use super::super::super::houseblocks_v1::common::Payload;
use anyhow::{bail, Error};
use std::slice;

pub trait Parser {
    fn get_byte(&mut self) -> Option<u8>;
    fn expect_end(&self) -> Result<(), Error>;

    fn expect_byte(&mut self) -> Result<u8, Error> {
        let byte = match self.get_byte() {
            Some(byte) => byte,
            None => bail!("premature data end"),
        };
        Ok(byte)
    }
    fn expect_bool(&mut self) -> Result<bool, Error> {
        let value = match self.expect_byte()? {
            b'0' => false,
            b'1' => true,
            value => bail!("invalid character for bool: {}", value),
        };
        Ok(value)
    }
    fn expect_u8(&mut self) -> Result<u8, Error> {
        let value_hex = [self.expect_byte()?, self.expect_byte()?];
        let mut value = [0u8; 1];
        hex::decode_to_slice(&value_hex, &mut value)?;
        Ok(u8::from_be_bytes(value))
    }
    fn expect_u16(&mut self) -> Result<u16, Error> {
        let value_hex = [
            self.expect_byte()?,
            self.expect_byte()?,
            self.expect_byte()?,
            self.expect_byte()?,
        ];
        let mut value = [0u8; 2];
        hex::decode_to_slice(&value_hex, &mut value)?;
        Ok(u16::from_be_bytes(value))
    }
}
pub struct ParserPayload<'a> {
    iterator: slice::Iter<'a, u8>,
}
impl<'a> ParserPayload<'a> {
    pub fn new(payload: &'a Payload) -> Self {
        Self {
            iterator: payload.as_bytes().iter(),
        }
    }
}
impl<'a> Parser for ParserPayload<'a> {
    fn get_byte(&mut self) -> Option<u8> {
        self.iterator.next().copied()
    }
    fn expect_end(&self) -> Result<(), Error> {
        if !self.iterator.is_empty() {
            bail!("more data available");
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests_parser_payload {
    use super::{super::super::super::houseblocks_v1::common::Payload, Parser, ParserPayload};

    #[test]
    fn get_byte_1() {
        let payload = Payload::new(Box::from(*b"")).unwrap();
        let mut parser = ParserPayload::new(&payload);
        assert_eq!(parser.get_byte(), None);
    }
    #[test]
    fn get_byte_2() {
        let payload = Payload::new(Box::from(*b"AZ")).unwrap();
        let mut parser = ParserPayload::new(&payload);
        assert_eq!(parser.get_byte(), Some(b'A'));
        assert_eq!(parser.get_byte(), Some(b'Z'));
        assert_eq!(parser.get_byte(), None);
        assert_eq!(parser.get_byte(), None);
    }
    #[test]
    fn expect_end_1() {
        let payload = Payload::new(Box::from(*b"")).unwrap();
        let mut parser = ParserPayload::new(&payload);
        assert_eq!(parser.get_byte(), None);
        assert!(parser.expect_end().is_ok());
    }
    #[test]
    fn expect_end_2() {
        let payload = Payload::new(Box::from(*b"AZ")).unwrap();
        let mut parser = ParserPayload::new(&payload);
        assert!(parser.expect_end().is_err());
        assert_eq!(parser.get_byte(), Some(b'A'));
        assert_eq!(parser.get_byte(), Some(b'Z'));
        assert!(parser.expect_end().is_ok());
    }
    #[test]
    fn expect_byte_1() {
        let payload = Payload::new(Box::from(*b"")).unwrap();
        let mut parser = ParserPayload::new(&payload);
        assert!(parser.expect_byte().is_err());
    }
    #[test]
    fn expect_byte_2() {
        let payload = Payload::new(Box::from(*b"AZ")).unwrap();
        let mut parser = ParserPayload::new(&payload);
        assert_eq!(parser.expect_byte().unwrap(), b'A');
        assert_eq!(parser.expect_byte().unwrap(), b'Z');
        assert!(parser.expect_byte().is_err());
        assert!(parser.expect_byte().is_err());
    }
    #[test]
    fn expect_u8_1() {
        let payload = Payload::new(Box::from(*b"GG")).unwrap();
        let mut parser = ParserPayload::new(&payload);
        assert!(parser.expect_u8().is_err());
    }
    #[test]
    fn expect_u8_2() {
        let payload = Payload::new(Box::from(*b"0")).unwrap();
        let mut parser = ParserPayload::new(&payload);
        assert!(parser.expect_u8().is_err());
    }
    #[test]
    fn expect_u8_3() {
        let payload = Payload::new(Box::from(*b"00FF")).unwrap();
        let mut parser = ParserPayload::new(&payload);
        assert_eq!(parser.expect_u8().unwrap(), 0);
        assert_eq!(parser.expect_u8().unwrap(), 255);
        assert!(parser.expect_u8().is_err());
    }
    #[test]
    fn expect_u8_4() {
        let payload = Payload::new(Box::from(*b"00FFAA123445EE")).unwrap();
        let mut parser = ParserPayload::new(&payload);
        assert_eq!(parser.expect_u8().unwrap(), 0);
        assert_eq!(parser.expect_u8().unwrap(), 255);
        assert_eq!(parser.expect_u8().unwrap(), 0xAA);
        assert_eq!(parser.expect_u8().unwrap(), 0x12);
        assert_eq!(parser.expect_u8().unwrap(), 0x34);
        assert_eq!(parser.expect_u8().unwrap(), 0x45);
        assert_eq!(parser.expect_u8().unwrap(), 0xEE);
        assert!(parser.expect_u8().is_err());
    }
    fn expect_u16_1() {
        let payload = Payload::new(Box::from(*b"0000FFFF1234EDCB")).unwrap();
        let mut parser = ParserPayload::new(&payload);
        assert_eq!(parser.expect_u16().unwrap(), 0x0000);
        assert_eq!(parser.expect_u16().unwrap(), 0xFFFF);
        assert_eq!(parser.expect_u16().unwrap(), 0x1234);
        assert_eq!(parser.expect_u16().unwrap(), 0xEDCB);
        assert!(parser.expect_u16().is_err());
    }
}
