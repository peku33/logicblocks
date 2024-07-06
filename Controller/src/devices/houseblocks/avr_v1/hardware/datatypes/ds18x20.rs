use super::super::{
    super::datatypes::ds18x20::{SensorType, State},
    parser::Parser,
};
use crate::datatypes::temperature::{Temperature, Unit};
use anyhow::{Context, Error};
use std::mem::transmute;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SensorState {
    inner: State,
}
impl SensorState {
    pub fn into_inner(self) -> State {
        self.inner
    }

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
                // Normally, left bits are used for sign but we cut it during transmission to
                // reduce space Bits 15:11 are always equal, so we can use 11
                // only
                let mut temperature = value & 0b0000_1111_1111_1111;
                if temperature & 0b0000_1000_0000_0000 != 0 {
                    temperature |= 0b1111_0000_0000_0000;
                }
                let temperature = unsafe { transmute::<u16, i16>(temperature) } as f64 / 16.0;
                let temperature = Temperature::from_unit(Unit::Celsius, temperature).unwrap();
                Some(temperature)
            }
            _ => None,
        };

        let state = State {
            sensor_type,
            reset_count,
            temperature,
        };

        Ok(Self { inner: state })
    }
}

#[cfg(test)]
mod tests_state {
    use super::*;

    #[test]
    fn invalid_1() {
        let state = SensorState::from_u16(0b0000_0000_0000_0000)
            .unwrap()
            .into_inner();
        let state_expected = State {
            sensor_type: SensorType::Empty,
            reset_count: 0,
            temperature: None,
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn invalid_2() {
        let state = SensorState::from_u16(0b0111_0111_1101_0000)
            .unwrap()
            .into_inner();
        let state_expected = State {
            sensor_type: SensorType::Invalid,
            reset_count: 3,
            temperature: None,
        };
        assert_eq!(state, state_expected);
    }

    #[test]
    fn from_u16_1() {
        let state = SensorState::from_u16(0b1000_0111_1101_0000)
            .unwrap()
            .into_inner();
        let state_expected = State {
            sensor_type: SensorType::S,
            reset_count: 0,
            temperature: Some(Temperature::from_unit(Unit::Celsius, 125.0).unwrap()),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_2() {
        let state = SensorState::from_u16(0b1100_0101_0101_0000)
            .unwrap()
            .into_inner();
        let state_expected = State {
            sensor_type: SensorType::B,
            reset_count: 0,
            temperature: Some(Temperature::from_unit(Unit::Celsius, 85.0).unwrap()),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_3() {
        let state = SensorState::from_u16(0b1001_0001_1001_0001)
            .unwrap()
            .into_inner();
        let state_expected = State {
            sensor_type: SensorType::S,
            reset_count: 1,
            temperature: Some(Temperature::from_unit(Unit::Celsius, 25.0625).unwrap()),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_4() {
        let state = SensorState::from_u16(0b1101_0000_1010_0010)
            .unwrap()
            .into_inner();
        let state_expected = State {
            sensor_type: SensorType::B,
            reset_count: 1,
            temperature: Some(Temperature::from_unit(Unit::Celsius, 10.125).unwrap()),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_5() {
        let state = SensorState::from_u16(0b1010_0000_0000_1000)
            .unwrap()
            .into_inner();
        let state_expected = State {
            sensor_type: SensorType::S,
            reset_count: 2,
            temperature: Some(Temperature::from_unit(Unit::Celsius, 0.5).unwrap()),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_6() {
        let state = SensorState::from_u16(0b1110_0000_0000_0000)
            .unwrap()
            .into_inner();
        let state_expected = State {
            sensor_type: SensorType::B,
            reset_count: 2,
            temperature: Some(Temperature::from_unit(Unit::Celsius, 0.0).unwrap()),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_7() {
        let state = SensorState::from_u16(0b1011_1111_1111_1000)
            .unwrap()
            .into_inner();
        let state_expected = State {
            sensor_type: SensorType::S,
            reset_count: 3,
            temperature: Some(Temperature::from_unit(Unit::Celsius, -0.5).unwrap()),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_8() {
        let state = SensorState::from_u16(0b1111_1111_0101_1110)
            .unwrap()
            .into_inner();
        let state_expected = State {
            sensor_type: SensorType::B,
            reset_count: 3,
            temperature: Some(Temperature::from_unit(Unit::Celsius, -10.125).unwrap()),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_9() {
        let state = SensorState::from_u16(0b1000_1110_0110_1111)
            .unwrap()
            .into_inner();
        let state_expected = State {
            sensor_type: SensorType::S,
            reset_count: 0,
            temperature: Some(Temperature::from_unit(Unit::Celsius, -25.0625).unwrap()),
        };
        assert_eq!(state, state_expected);
    }
    #[test]
    fn from_u16_10() {
        let state = SensorState::from_u16(0b1100_1100_1001_0000)
            .unwrap()
            .into_inner();
        let state_expected = State {
            sensor_type: SensorType::B,
            reset_count: 0,
            temperature: Some(Temperature::from_unit(Unit::Celsius, -55.0).unwrap()),
        };
        assert_eq!(state, state_expected);
    }
}
