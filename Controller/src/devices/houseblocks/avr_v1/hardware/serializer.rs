use super::super::super::houseblocks_v1::common::Payload;
use arrayvec::ArrayVec;

pub struct Serializer {
    container: Vec<u8>,
}
impl Serializer {
    pub fn new() -> Self {
        Self {
            container: Vec::<u8>::new(),
        }
    }
    pub fn into_payload(self) -> Payload {
        Payload::new(self.container.into_boxed_slice()).unwrap()
    }
    pub fn push_byte(
        &mut self,
        value: u8,
    ) {
        self.container.push(value);
    }
    pub fn push_bool(
        &mut self,
        value: bool,
    ) {
        self.push_byte(if value { b'1' } else { b'0' })
    }
    pub fn push_u8(
        &mut self,
        value: u8,
    ) {
        let mut buffer_hex = [0u8; 2];
        hex::encode_to_slice(&value.to_be_bytes(), &mut buffer_hex).unwrap();
        for item_hex in buffer_hex.iter_mut() {
            *item_hex = item_hex.to_ascii_uppercase();
        }
        self.container.extend(buffer_hex.iter());
    }
    pub fn push_u16(
        &mut self,
        value: u16,
    ) {
        let mut buffer_hex = [0u8; 4];
        hex::encode_to_slice(&value.to_be_bytes(), &mut buffer_hex).unwrap();
        for item_hex in buffer_hex.iter_mut() {
            *item_hex = item_hex.to_ascii_uppercase();
        }
        self.container.extend(buffer_hex.iter());
    }
    pub fn push_bool_array_8(
        &mut self,
        value: ArrayVec<bool, 8>,
    ) {
        let mut bits = 0u8;
        for (index, item) in value.into_iter().enumerate() {
            if item {
                bits |= 1 << index;
            }
        }
        self.push_u8(bits);
    }
    pub fn push_bool_array_16(
        &mut self,
        value: ArrayVec<bool, 16>,
    ) {
        let mut bits = 0u16;
        for (index, item) in value.into_iter().enumerate() {
            if item {
                bits |= 1 << index;
            }
        }
        self.push_u16(bits);
    }
}
#[cfg(test)]
mod tests_serializer {
    use super::{super::super::super::houseblocks_v1::common::Payload, Serializer};
    #[test]
    fn empty() {
        let serializer = Serializer::new();
        let payload = serializer.into_payload();
        assert_eq!(payload, Payload::new(Box::from(*b"")).unwrap());
    }
    #[test]
    fn push_byte_1() {
        let mut serializer = Serializer::new();
        serializer.push_byte(b'A');
        let payload = serializer.into_payload();
        assert_eq!(payload, Payload::new(Box::from(*b"A")).unwrap());
    }
    #[test]
    fn push_byte_2() {
        let mut serializer = Serializer::new();
        serializer.push_byte(b'A');
        serializer.push_byte(b'B');
        let payload = serializer.into_payload();
        assert_eq!(payload, Payload::new(Box::from(*b"AB")).unwrap());
    }
    #[test]
    fn push_bool_1() {
        let mut serializer = Serializer::new();
        serializer.push_bool(true);
        serializer.push_bool(false);
        serializer.push_bool(false);
        serializer.push_bool(true);
        serializer.push_bool(true);
        serializer.push_bool(true);
        serializer.push_bool(false);
        serializer.push_bool(true);
        serializer.push_bool(false);
        let payload = serializer.into_payload();
        assert_eq!(payload, Payload::new(Box::from(*b"100111010")).unwrap());
    }
    #[test]
    fn push_u8_1() {
        let mut serializer = Serializer::new();
        serializer.push_u8(0);
        serializer.push_u8(255);
        serializer.push_u8(0xAA);
        serializer.push_u8(0x12);
        serializer.push_u8(0x34);
        serializer.push_u8(0x45);
        serializer.push_u8(0xEE);
        let payload = serializer.into_payload();
        assert_eq!(
            payload,
            Payload::new(Box::from(*b"00FFAA123445EE")).unwrap()
        );
    }
    #[test]
    fn push_u16_1() {
        let mut serializer = Serializer::new();
        serializer.push_u16(0x0000);
        serializer.push_u16(0xFFFF);
        serializer.push_u16(0x1234);
        serializer.push_u16(0xEDCB);
        let payload = serializer.into_payload();
        assert_eq!(
            payload,
            Payload::new(Box::from(*b"0000FFFF1234EDCB")).unwrap()
        );
    }
    #[test]
    fn push_bool_array_8() {
        let mut serializer = Serializer::new();

        serializer.push_bool_array_8([true, true, false, false, false, true, false, true].into());
        let payload = serializer.into_payload();
        assert_eq!(payload, Payload::new(Box::from(*b"A3")).unwrap());
    }
    #[test]
    fn push_bool_array_16() {
        let mut serializer = Serializer::new();

        serializer.push_bool_array_16(
            [
                false, true, false, false, false, false, true, true, false, false, false, false,
                false, false, false, true,
            ]
            .into(),
        );
        let payload = serializer.into_payload();
        assert_eq!(payload, Payload::new(Box::from(*b"80C2")).unwrap());
    }
}
