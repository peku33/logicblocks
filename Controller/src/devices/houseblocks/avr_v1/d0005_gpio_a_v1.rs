pub mod hardware {
    use super::super::{
        super::houseblocks_v1::common::{AddressDeviceType, Payload},
        hardware::{
            common::ds18x20, driver::ApplicationDriver, parser::Parser, property, runner,
            serializer::Serializer,
        },
    };
    use crate::datatypes::{color_rgb_boolean::ColorRgbBoolean, voltage::Voltage};
    use anyhow::{bail, ensure, Context, Error};
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use std::{iter, time::Duration};

    // block configuration
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Block1Function {
        Unused,
        AnalogIn,
        DigitalIn,
        DigitalOut,
    }
    pub const BLOCK_1_SIZE: usize = 4;
    pub type Block1Functions = [Block1Function; BLOCK_1_SIZE];

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Block2Function {
        Unused,
        DigitalIn,
        DigitalOut,
        Ds18x20,
    }
    pub const BLOCK_2_SIZE: usize = 4;
    pub type Block2Functions = [Block2Function; BLOCK_2_SIZE];

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Block3Function {
        Unused,
        AnalogIn,
    }
    pub const BLOCK_3_SIZE: usize = 2;
    pub type Block3Functions = [Block3Function; BLOCK_3_SIZE];

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Block4Function {
        Unused,
        DigitalOut,
    }
    pub const BLOCK_4_SIZE: usize = 3;
    pub type Block4Functions = [Block4Function; BLOCK_4_SIZE];

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct BlockFunctions {
        pub block_1_functions: Block1Functions,
        pub block_2_functions: Block2Functions,
        pub block_3_functions: Block3Functions,
        pub block_4_functions: Block4Functions,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct BlockFunctionsReversed {
        pub analog_in_any: bool,
        pub digital_in_any: bool,
        pub digital_out_any: bool,
        pub ds18x20_any: bool,
    }
    impl BlockFunctionsReversed {
        pub fn new(block_functions: &BlockFunctions) -> Self {
            let analog_in_any = false
                || block_functions
                    .block_1_functions
                    .iter()
                    .any(|block_1_function| *block_1_function == Block1Function::AnalogIn)
                || block_functions
                    .block_3_functions
                    .iter()
                    .any(|block_3_function| *block_3_function == Block3Function::AnalogIn);

            let digital_in_any = false
                || block_functions
                    .block_1_functions
                    .iter()
                    .any(|block_1_function| *block_1_function == Block1Function::DigitalIn)
                || block_functions
                    .block_2_functions
                    .iter()
                    .any(|block_2_function| *block_2_function == Block2Function::DigitalIn);

            let digital_out_any = false
                || block_functions
                    .block_1_functions
                    .iter()
                    .any(|block_1_function| *block_1_function == Block1Function::DigitalOut)
                || block_functions
                    .block_2_functions
                    .iter()
                    .any(|block_2_function| *block_2_function == Block2Function::DigitalOut)
                || block_functions
                    .block_4_functions
                    .iter()
                    .any(|block_4_function| *block_4_function == Block4Function::DigitalOut);

            let ds18x20_any = false
                || block_functions
                    .block_2_functions
                    .iter()
                    .any(|block_2_function| *block_2_function == Block2Function::Ds18x20);

            Self {
                analog_in_any,
                digital_in_any,
                digital_out_any,
                ds18x20_any,
            }
        }
    }

    // properties
    pub type StatusLedValue = ColorRgbBoolean;

    pub type AnalogInValue = Voltage;
    pub const ANALOG_IN_COUNT: usize = BLOCK_1_SIZE + BLOCK_3_SIZE;
    pub type AnalogInValues = [AnalogInValue; ANALOG_IN_COUNT];

    pub type DigitalInValue = bool;
    pub const DIGITAL_IN_COUNT: usize = BLOCK_1_SIZE + BLOCK_2_SIZE;
    pub type DigitalInValues = [DigitalInValue; DIGITAL_IN_COUNT];

    pub type DigitalOutValue = bool;
    pub const DIGITAL_OUT_COUNT: usize = BLOCK_1_SIZE + BLOCK_2_SIZE + BLOCK_4_SIZE;
    pub type DigitalOutValues = [DigitalOutValue; DIGITAL_OUT_COUNT];

    pub type Ds18x20Value = ds18x20::State;
    pub const DS18X20_COUNT: usize = BLOCK_2_SIZE;
    pub type Ds18x20Values = [Ds18x20Value; DS18X20_COUNT];

    #[derive(Debug)]
    pub struct Properties {
        status_led: property::state_out::Property<StatusLedValue>,
        analog_in: property::state_in::Property<AnalogInValues>,
        digital_in: property::state_in::Property<DigitalInValues>,
        digital_out: property::state_out::Property<DigitalOutValues>,
        ds18x20: property::state_in::Property<Ds18x20Values>,
    }
    impl Properties {
        pub fn new() -> Self {
            Self {
                status_led: property::state_out::Property::<StatusLedValue>::new(
                    ColorRgbBoolean::off(),
                ),
                analog_in: property::state_in::Property::<AnalogInValues>::new(),
                digital_in: property::state_in::Property::<DigitalInValues>::new(),
                digital_out: property::state_out::Property::<DigitalOutValues>::new(
                    [false; DIGITAL_OUT_COUNT],
                ),
                ds18x20: property::state_in::Property::<Ds18x20Values>::new(),
            }
        }
    }
    impl runner::Properties for Properties {
        fn user_pending(&self) -> bool {
            false
                || self.analog_in.user_pending()
                || self.digital_in.user_pending()
                || self.ds18x20.user_pending()
        }
        fn device_reset(&self) {
            self.status_led.device_reset();
            self.analog_in.device_reset();
            self.digital_in.device_reset();
            self.digital_out.device_reset();
            self.ds18x20.device_reset();
        }

        type Remote = PropertiesRemote;
        fn remote(&self) -> Self::Remote {
            PropertiesRemote {
                status_led: self.status_led.user_sink(),
                analog_in: self.analog_in.user_stream(),
                digital_in: self.digital_in.user_stream(),
                digital_out: self.digital_out.user_sink(),
                ds18x20: self.ds18x20.user_stream(),
            }
        }
    }
    #[derive(Debug)]
    pub struct PropertiesRemote {
        pub status_led: property::state_out::Sink<StatusLedValue>,
        pub analog_in: property::state_in::Stream<AnalogInValues>,
        pub digital_in: property::state_in::Stream<DigitalInValues>,
        pub digital_out: property::state_out::Sink<DigitalOutValues>,
        pub ds18x20: property::state_in::Stream<Ds18x20Values>,
    }

    // device
    #[derive(Copy, Clone, Debug)]
    pub struct Configuration {
        pub block_functions: BlockFunctions,
    }

    #[derive(Debug)]
    pub struct Device {
        configuration: Configuration,
        block_functions_reversed: BlockFunctionsReversed,

        properties: Properties,
    }
    impl Device {
        pub fn new(configuration: Configuration) -> Self {
            let block_functions_reversed =
                BlockFunctionsReversed::new(&configuration.block_functions);

            Self {
                configuration,
                block_functions_reversed,
                properties: Properties::new(),
            }
        }
    }
    impl runner::Device for Device {
        fn device_type_name() -> &'static str {
            "GPIO_A_v1"
        }
        fn address_device_type() -> AddressDeviceType {
            AddressDeviceType::new_from_ordinal(5).unwrap()
        }

        type Properties = Properties;
        fn properties(&self) -> &Self::Properties {
            &self.properties
        }
    }
    #[async_trait]
    impl runner::BusDevice for Device {
        async fn initialize(
            &self,
            driver: &ApplicationDriver<'_>,
        ) -> Result<(), Error> {
            let request = BusRequest {
                configuration: Some(BusRequestConfiguration {
                    block_functions: self.configuration.block_functions,
                }),
                status_led: None,
                poll_request: false,
                analog_in_request: false,
                digital_in_request: false,
                digital_out: None,
                ds18x20_request: false,
            };
            let request_payload = request.to_payload();
            let response_payload = driver
                .transaction_out_in(request_payload, None)
                .await
                .context("transaction_out_in")?;
            let response = BusResponse::from_payload(&response_payload).context("from_payload")?;

            ensure!(
                response
                    == BusResponse {
                        poll: None,
                        analog_in: None,
                        digital_in: None,
                        ds18x20: None
                    }
            );

            Ok(())
        }

        fn poll_delay(&self) -> Option<Duration> {
            // analog in, digital in -> poll in 250ms
            // ds18x20 -> poll in 1000ms
            // none -> no poll

            if false
                || self.block_functions_reversed.analog_in_any
                || self.block_functions_reversed.digital_in_any
            {
                return Some(Duration::from_millis(250));
            }
            if self.block_functions_reversed.ds18x20_any {
                return Some(Duration::from_millis(1000));
            }

            None
        }
        async fn poll(
            &self,
            driver: &ApplicationDriver<'_>,
        ) -> Result<(), Error> {
            // stage 1 - poll + read fast values
            let status_led_request = self.properties.status_led.device_pending();
            let digital_out_request = if self.block_functions_reversed.digital_out_any {
                self.properties.digital_out.device_pending()
            } else {
                None
            };

            let stage_1_request = BusRequest {
                configuration: None,
                status_led: status_led_request.as_ref().map(|status_led_request| {
                    BusRequestStatusLed {
                        value: **status_led_request,
                    }
                }),
                poll_request: false
                    || self.block_functions_reversed.analog_in_any
                    || self.block_functions_reversed.ds18x20_any,
                analog_in_request: false,
                digital_in_request: self.block_functions_reversed.digital_in_any
                    && self.properties.digital_in.device_must_read(),
                digital_out: digital_out_request.as_ref().map(|digital_out_request| {
                    BusRequestDigitalOut {
                        values: **digital_out_request,
                    }
                }),
                ds18x20_request: false,
            };
            // stage 1 request is always used, to check device uptime
            let stage_1_request_payload = stage_1_request.to_payload();
            let stage_1_response_payload = driver
                .transaction_out_in(stage_1_request_payload, None)
                .await
                .context("transaction_out_in stage 1")?;
            let stage_1_response = BusResponse::from_payload(&stage_1_response_payload)
                .context("from_payload stage 1")?;

            if let Some(status_led_request) = status_led_request {
                status_led_request.commit();
            }
            if let Some(digital_out_request) = digital_out_request {
                digital_out_request.commit();
            }

            let BusResponse {
                poll: stage_1_response_poll,
                analog_in: stage_1_response_analog_in,
                digital_in: stage_1_response_digital_in,
                ds18x20: stage_1_response_ds18x20,
            } = stage_1_response;

            let stage_1_response_poll = match (stage_1_request.poll_request, stage_1_response_poll)
            {
                (false, None) => None,
                (true, Some(stage_1_response_poll)) => Some(stage_1_response_poll),
                _ => bail!("poll mismatch"),
            };
            ensure!(stage_1_response_analog_in.is_none());
            let stage_1_response_digital_in = match (
                stage_1_request.digital_in_request,
                stage_1_response_digital_in,
            ) {
                (false, None) => None,
                (true, Some(stage_1_response_digital_in)) => Some(stage_1_response_digital_in),
                _ => bail!("digital_in mismatch"),
            };
            ensure!(stage_1_response_ds18x20.is_none());

            if let Some(stage_1_response_digital_in) = stage_1_response_digital_in {
                self.properties
                    .digital_in
                    .device_set(stage_1_response_digital_in.values);
            }

            // stage 2 - get additional data
            let stage_2_request = BusRequest {
                configuration: None,
                status_led: None,
                poll_request: false,
                analog_in_request: self.block_functions_reversed.analog_in_any
                    && (false
                        || self.properties.analog_in.device_must_read()
                        || stage_1_response_poll
                            .as_ref()
                            .map(|stage_1_response_poll| stage_1_response_poll.analog_in)
                            .unwrap_or(false)),
                digital_in_request: false,
                digital_out: None,
                ds18x20_request: self.block_functions_reversed.ds18x20_any
                    && (false
                        || self.properties.ds18x20.device_must_read()
                        || stage_1_response_poll
                            .as_ref()
                            .map(|stage_1_response_poll| stage_1_response_poll.ds18x20)
                            .unwrap_or(false)),
            };
            if stage_2_request.is_nop() {
                return Ok(());
            }
            let stage_2_request_payload = stage_2_request.to_payload();
            let stage_2_response_payload = driver
                .transaction_out_in(stage_2_request_payload, None)
                .await
                .context("transaction_out_in stage 2")?;
            let stage_2_response = BusResponse::from_payload(&stage_2_response_payload)
                .context("from_payload stage 2")?;

            let BusResponse {
                poll: stage_2_response_poll,
                analog_in: stage_2_response_analog_in,
                digital_in: stage_2_response_digital_in,
                ds18x20: stage_2_response_ds18x20,
            } = stage_2_response;

            ensure!(stage_2_response_poll.is_none());
            let stage_2_response_analog_in = match (
                stage_2_request.analog_in_request,
                stage_2_response_analog_in,
            ) {
                (false, None) => None,
                (true, Some(stage_2_response_analog_in)) => Some(stage_2_response_analog_in),
                _ => bail!("analog_in mismatch"),
            };
            ensure!(stage_2_response_digital_in.is_none());
            let stage_2_response_ds18x20 =
                match (stage_2_request.ds18x20_request, stage_2_response_ds18x20) {
                    (false, None) => None,
                    (true, Some(stage_2_response_ds18x20)) => Some(stage_2_response_ds18x20),
                    _ => bail!("ds18x20 mismatch"),
                };

            if let Some(stage_2_response_analog_in) = stage_2_response_analog_in {
                self.properties
                    .analog_in
                    .device_set(stage_2_response_analog_in.values);
            }
            if let Some(stage_2_response_ds18x20) = stage_2_response_ds18x20 {
                self.properties
                    .ds18x20
                    .device_set(stage_2_response_ds18x20.values);
            }

            Ok(())
        }

        async fn deinitialize(
            &self,
            _driver: &ApplicationDriver<'_>,
        ) -> Result<(), Error> {
            Ok(())
        }

        fn failed(&self) {}
    }

    // bus
    #[derive(PartialEq, Eq, Debug)]
    struct BusRequestConfiguration {
        pub block_functions: BlockFunctions,
    }
    impl BusRequestConfiguration {
        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            for block_1_function in self.block_functions.block_1_functions.iter() {
                let byte = match block_1_function {
                    Block1Function::Unused => b'A',
                    Block1Function::AnalogIn => b'A',
                    Block1Function::DigitalIn => b'I',
                    Block1Function::DigitalOut => b'O',
                };
                serializer.push_byte(byte);
            }
            for block_2_function in self.block_functions.block_2_functions.iter() {
                let byte = match block_2_function {
                    Block2Function::Unused => b'I',
                    Block2Function::DigitalIn => b'I',
                    Block2Function::DigitalOut => b'O',
                    Block2Function::Ds18x20 => b'T',
                };
                serializer.push_byte(byte);
            }
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusRequestStatusLed {
        pub value: StatusLedValue,
    }
    impl BusRequestStatusLed {
        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            serializer.push_bool(self.value.r);
            serializer.push_bool(self.value.g);
            serializer.push_bool(self.value.b);
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusRequestDigitalOut {
        pub values: DigitalOutValues,
    }
    impl BusRequestDigitalOut {
        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            let values = iter::empty()
                .chain(self.values.iter().copied())
                .chain(iter::repeat(false))
                .take(16)
                .collect::<ArrayVec<bool, 16>>()
                .into_inner()
                .unwrap();
            serializer.push_bool_array_16(values);
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusRequest {
        pub configuration: Option<BusRequestConfiguration>,
        pub status_led: Option<BusRequestStatusLed>,
        pub poll_request: bool,
        pub analog_in_request: bool,
        pub digital_in_request: bool,
        pub digital_out: Option<BusRequestDigitalOut>,
        pub ds18x20_request: bool,
    }
    impl BusRequest {
        pub fn is_nop(&self) -> bool {
            *self
                == (Self {
                    configuration: None,
                    status_led: None,
                    poll_request: false,
                    analog_in_request: false,
                    digital_in_request: false,
                    digital_out: None,
                    ds18x20_request: false,
                })
        }

        pub fn to_payload(&self) -> Payload {
            let mut serializer = Serializer::new();
            self.serialize(&mut serializer);
            serializer.into_payload()
        }

        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            if let Some(configuration) = self.configuration.as_ref() {
                serializer.push_byte(b'C');
                configuration.serialize(serializer);
            }
            if let Some(status_led) = self.status_led.as_ref() {
                serializer.push_byte(b'L');
                status_led.serialize(serializer);
            }
            if self.poll_request {
                serializer.push_byte(b'P');
            }
            if self.analog_in_request {
                serializer.push_byte(b'A');
            }
            if self.digital_in_request {
                serializer.push_byte(b'I');
            }
            if let Some(digital_out) = self.digital_out.as_ref() {
                serializer.push_byte(b'O');
                digital_out.serialize(serializer);
            }
            if self.ds18x20_request {
                serializer.push_byte(b'T');
            }
        }
    }
    #[cfg(test)]
    mod tests_bus_request {
        use super::{
            super::super::super::houseblocks_v1::common::Payload, Block1Function, Block2Function,
            Block3Function, Block4Function, BlockFunctions, BusRequest, BusRequestConfiguration,
            BusRequestDigitalOut, BusRequestStatusLed, StatusLedValue,
        };

        #[test]
        fn empty() {
            let request = BusRequest {
                configuration: None,
                status_led: None,
                poll_request: false,
                analog_in_request: false,
                digital_in_request: false,
                digital_out: None,
                ds18x20_request: false,
            };
            let payload = request.to_payload();

            let payload_expected = Payload::new(Box::from(*b"")).unwrap();

            assert_eq!(payload, payload_expected);
        }

        #[test]
        fn everything() {
            let request = BusRequest {
                configuration: Some(BusRequestConfiguration {
                    block_functions: BlockFunctions {
                        block_1_functions: [
                            Block1Function::Unused,
                            Block1Function::AnalogIn,
                            Block1Function::DigitalIn,
                            Block1Function::DigitalOut,
                        ],
                        block_2_functions: [
                            Block2Function::Unused,
                            Block2Function::DigitalIn,
                            Block2Function::DigitalOut,
                            Block2Function::Ds18x20,
                        ],
                        block_3_functions: [
                            Block3Function::Unused,
                            Block3Function::AnalogIn,
                            // break
                        ],
                        block_4_functions: [
                            Block4Function::Unused,
                            Block4Function::DigitalOut,
                            Block4Function::Unused,
                        ],
                    },
                }),
                status_led: Some(BusRequestStatusLed {
                    value: StatusLedValue {
                        r: true,
                        g: true,
                        b: true,
                    },
                }),
                poll_request: true,
                analog_in_request: true,
                digital_in_request: true,
                digital_out: Some(BusRequestDigitalOut {
                    values: [
                        true, true, true, true, true, true, true, true, true, true, true,
                    ],
                }),
                ds18x20_request: true,
            };
            let payload = request.to_payload();

            let payload_expected = Payload::new(Box::from(*b"CAAIOIIOTL111PAIO07FFT")).unwrap();

            assert_eq!(payload, payload_expected);
        }

        #[test]
        fn request_1() {
            let request = BusRequest {
                configuration: None,
                status_led: Some(BusRequestStatusLed {
                    value: StatusLedValue {
                        r: false,
                        g: true,
                        b: true,
                    },
                }),
                poll_request: false,
                analog_in_request: false,
                digital_in_request: false,
                digital_out: Some(BusRequestDigitalOut {
                    values: [
                        true, true, false, true, false, false, true, false, false, true, true,
                    ],
                }),
                ds18x20_request: false,
            };
            let payload = request.to_payload();

            let payload_expected = Payload::new(Box::from(*b"L011O064B")).unwrap();

            assert_eq!(payload, payload_expected);
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponsePoll {
        pub analog_in: bool,
        pub ds18x20: bool,
    }
    impl BusResponsePoll {
        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let analog_in = parser.expect_bool().context("analog_in")?;
            let ds18x20 = parser.expect_bool().context("ds18x20")?;
            Ok(Self { analog_in, ds18x20 })
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponseAnalogIn {
        pub values: AnalogInValues,
    }
    impl BusResponseAnalogIn {
        fn transform_block(
            value: u16,
            index: usize,
        ) -> Result<Voltage, Error> {
            // it should be 1023 actually (10 bits)
            // but for good division in tests we keep it at 1024
            ensure!((0..=1024).contains(&value));

            let multiplier = if (0..BLOCK_1_SIZE).contains(&index) {
                5.0
            } else if (BLOCK_1_SIZE..(BLOCK_1_SIZE + BLOCK_3_SIZE)).contains(&index) {
                26.0
            } else {
                bail!("index out of bounds");
            };

            let analog_in_value = Voltage::from_volts((value as f64) / 1024.0 * multiplier);
            Ok(analog_in_value)
        }

        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let values = (0..ANALOG_IN_COUNT)
                .map(|index| -> Result<_, Error> {
                    let value = parser.expect_u16().context("expect_u16")?;
                    let value = Self::transform_block(value, index).context("transform_block")?;
                    Ok(value)
                })
                .collect::<Result<ArrayVec<Voltage, ANALOG_IN_COUNT>, Error>>()
                .context("collect")?
                .into_inner()
                .unwrap();
            Ok(Self { values })
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponseDigitalIn {
        pub values: DigitalInValues,
    }
    impl BusResponseDigitalIn {
        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let values = parser
                .expect_bool_array_8()
                .context("expect_bool_array_8")?;
            Ok(Self { values })
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponseDs18x20 {
        pub values: Ds18x20Values,
    }
    impl BusResponseDs18x20 {
        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let values = (0..DS18X20_COUNT)
                .map(|_| ds18x20::State::parse(parser))
                .collect::<Result<ArrayVec<_, { DS18X20_COUNT }>, _>>()
                .context("collect")?
                .into_inner()
                .unwrap();
            Ok(Self { values })
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponse {
        pub poll: Option<BusResponsePoll>,
        pub analog_in: Option<BusResponseAnalogIn>,
        pub digital_in: Option<BusResponseDigitalIn>,
        pub ds18x20: Option<BusResponseDs18x20>,
    }
    impl BusResponse {
        pub fn from_payload(payload: &Payload) -> Result<Self, Error> {
            let mut parser = Parser::from_payload(payload);
            let self_ = Self::parse(&mut parser).context("parse")?;
            Ok(self_)
        }

        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let mut poll: Option<BusResponsePoll> = None;
            let mut analog_in: Option<BusResponseAnalogIn> = None;
            let mut digital_in: Option<BusResponseDigitalIn> = None;
            let mut ds18x20: Option<BusResponseDs18x20> = None;

            while let Some(opcode) = parser.get_byte() {
                match opcode {
                    b'P' => {
                        let value = BusResponsePoll::parse(parser).context("poll")?;
                        ensure!(poll.replace(value).is_none(), "duplicated poll");
                    }
                    b'A' => {
                        let value = BusResponseAnalogIn::parse(parser).context("analog_in")?;
                        ensure!(analog_in.replace(value).is_none(), "duplicated analog_in");
                    }
                    b'I' => {
                        let value = BusResponseDigitalIn::parse(parser).context("digital_in")?;
                        ensure!(digital_in.replace(value).is_none(), "duplicated digital_in");
                    }
                    b'T' => {
                        let value = BusResponseDs18x20::parse(parser).context("ds18x20")?;
                        ensure!(ds18x20.replace(value).is_none(), "duplicated ds18x20");
                    }
                    opcode => bail!("unrecognized opcode: {}", opcode),
                }
            }

            parser.expect_end().context("expect_end")?;

            Ok(Self {
                poll,
                analog_in,
                digital_in,
                ds18x20,
            })
        }
    }
    #[cfg(test)]
    mod tests_bus_response {
        use super::{
            super::super::super::{
                avr_v1::hardware::common::ds18x20, houseblocks_v1::common::Payload,
            },
            BusResponse, BusResponseAnalogIn, BusResponseDigitalIn, BusResponseDs18x20,
            BusResponsePoll,
        };
        use crate::datatypes::{
            temperature::{Temperature, Unit as TemperatureUnit},
            voltage::Voltage,
        };

        #[test]
        fn empty() {
            let payload = Payload::new(Box::from(*b"")).unwrap();
            let bus_response = BusResponse::from_payload(&payload).unwrap();

            let bus_response_expected = BusResponse {
                poll: None,
                analog_in: None,
                digital_in: None,
                ds18x20: None,
            };

            assert_eq!(bus_response, bus_response_expected);
        }

        #[test]
        fn full() {
            let payload = Payload::new(Box::from(
                *b"P11A000001000200040002000400IA4T000087D09191CC90",
            ))
            .unwrap();
            let bus_response = BusResponse::from_payload(&payload).unwrap();

            let bus_response_expected = BusResponse {
                poll: Some(BusResponsePoll {
                    analog_in: true,
                    ds18x20: true,
                }),
                analog_in: Some(BusResponseAnalogIn {
                    values: [
                        Voltage::from_volts(0.0),
                        Voltage::from_volts(1.25),
                        Voltage::from_volts(2.5),
                        Voltage::from_volts(5.0),
                        Voltage::from_volts(13.0),
                        Voltage::from_volts(26.0),
                    ],
                }),
                digital_in: Some(BusResponseDigitalIn {
                    values: [false, false, true, false, false, true, false, true],
                }),
                ds18x20: Some(BusResponseDs18x20 {
                    values: [
                        ds18x20::State {
                            sensor_type: ds18x20::SensorType::Empty,
                            reset_count: 0,
                            temperature: None,
                        },
                        ds18x20::State {
                            sensor_type: ds18x20::SensorType::S,
                            reset_count: 0,
                            temperature: Some(Temperature::new(TemperatureUnit::Celsius, 125.0)),
                        },
                        ds18x20::State {
                            sensor_type: ds18x20::SensorType::S,
                            reset_count: 1,
                            temperature: Some(Temperature::new(TemperatureUnit::Celsius, 25.0625)),
                        },
                        ds18x20::State {
                            sensor_type: ds18x20::SensorType::B,
                            reset_count: 0,
                            temperature: Some(Temperature::new(TemperatureUnit::Celsius, -55.0)),
                        },
                    ],
                }),
            };

            assert_eq!(bus_response, bus_response_expected);
        }
    }
}
