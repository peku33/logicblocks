use super::super::parser::Parser;
use crate::datatypes::temperature::{Temperature, Unit};
use anyhow::{Context, Error};
use serde::Serialize;
use std::mem::transmute;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum SensorType {
    Empty,
    Invalid,
    S,
    B,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub struct State {
    pub sensor_type: SensorType,
    pub reset_count: u8,
    pub temperature: Option<Temperature>,
}
impl State {
    pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
        let value = parser.expect_u16().context("expect_u16")?;
        let value = Self::from_u16(value).context("from_u16")?;
        Ok(value)
    }

    pub fn from_u16(value: u16) -> Result<Self, Error> {
        let sensor_type = match (value >> 14) & 0b11 {
            0b00 => SensorType::Empty,
            0b01 => SensorType::Invalid,
            0b10 => SensorType::S,
            0b11 => SensorType::B,
            _ => panic!(),
        };

        let reset_count = ((value >> 12) & 0b11) as u8;

        let temperature = match sensor_type {
            SensorType::S | SensorType::B => {
                // Normally, left bits are used for sign but we cut it during transmission to reduce space
                // Bits 15:11 are always equal, so we can use 11 only
                let mut temperature = value & 0b0000_1111_1111_1111;
                if temperature & 0b0000_1000_0000_0000 != 0 {
                    temperature |= 0b1111_0000_0000_0000;
                }
                let temperature = unsafe { transmute::<u16, i16>(temperature) } as f64 / 16.0;
                let temperature = Temperature::new(Unit::Celsius, temperature);
                Some(temperature)
            }
            _ => None,
        };

        Ok(Self {
            sensor_type,
            reset_count,
            temperature,
        })
    }
}
#[cfg(test)]
mod tests_state {
    use super::*;

    #[test]
    fn invalid_1() {
        let state = State::from_u16(0b0000_0000_0000_0000).unwrap();
        let state_expected = State {
            sensor_type: SensorType::Empty,
            reset_count: 0,
            temperature: None,
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn invalid_2() {
        let state = State::from_u16(0b0111_0111_1101_0000).unwrap();
        let state_expected = State {
            sensor_type: SensorType::Invalid,
            reset_count: 3,
            temperature: None,
        };
        assert_eq!(state, state_expected);
    }

    #[test]
    fn from_u16_1() {
        let state = State::from_u16(0b1000_0111_1101_0000).unwrap();
        let state_expected = State {
            sensor_type: SensorType::S,
            reset_count: 0,
            temperature: Some(Temperature::new(Unit::Celsius, 125.0)),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_2() {
        let state = State::from_u16(0b1100_0101_0101_0000).unwrap();
        let state_expected = State {
            sensor_type: SensorType::B,
            reset_count: 0,
            temperature: Some(Temperature::new(Unit::Celsius, 85.0)),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_3() {
        let state = State::from_u16(0b1001_0001_1001_0001).unwrap();
        let state_expected = State {
            sensor_type: SensorType::S,
            reset_count: 1,
            temperature: Some(Temperature::new(Unit::Celsius, 25.0625)),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_4() {
        let state = State::from_u16(0b1101_0000_1010_0010).unwrap();
        let state_expected = State {
            sensor_type: SensorType::B,
            reset_count: 1,
            temperature: Some(Temperature::new(Unit::Celsius, 10.125)),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_5() {
        let state = State::from_u16(0b1010_0000_0000_1000).unwrap();
        let state_expected = State {
            sensor_type: SensorType::S,
            reset_count: 2,
            temperature: Some(Temperature::new(Unit::Celsius, 0.5)),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_6() {
        let state = State::from_u16(0b1110_0000_0000_0000).unwrap();
        let state_expected = State {
            sensor_type: SensorType::B,
            reset_count: 2,
            temperature: Some(Temperature::new(Unit::Celsius, 0.0)),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_7() {
        let state = State::from_u16(0b1011_1111_1111_1000).unwrap();
        let state_expected = State {
            sensor_type: SensorType::S,
            reset_count: 3,
            temperature: Some(Temperature::new(Unit::Celsius, -0.5)),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_8() {
        let state = State::from_u16(0b1111_1111_0101_1110).unwrap();
        let state_expected = State {
            sensor_type: SensorType::B,
            reset_count: 3,
            temperature: Some(Temperature::new(Unit::Celsius, -10.125)),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_9() {
        let state = State::from_u16(0b1000_1110_0110_1111).unwrap();
        let state_expected = State {
            sensor_type: SensorType::S,
            reset_count: 0,
            temperature: Some(Temperature::new(Unit::Celsius, -25.0625)),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_10() {
        let state = State::from_u16(0b1100_1100_1001_0000).unwrap();
        let state_expected = State {
            sensor_type: SensorType::B,
            reset_count: 0,
            temperature: Some(Temperature::new(Unit::Celsius, -55.0)),
        };
        assert_eq!(state, state_expected);
    }
}
