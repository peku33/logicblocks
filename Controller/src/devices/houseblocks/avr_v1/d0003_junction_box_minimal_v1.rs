pub mod logic {
    use super::{super::logic, hardware};
    use crate::{
        datatypes::{boolean::Boolean, temperature::Temperature, time_duration::TimeDuration},
        logic::{device::Signals, signal, signal::SignalBase},
        util::waker_stream,
        web::{
            sse_aggregated::{Node, NodeProvider},
            uri_cursor::{Handler, UriCursor},
            Request, Response,
        },
    };
    use array_init::array_init;
    use async_trait::async_trait;
    use futures::{
        future::{BoxFuture, FutureExt},
        pin_mut, select,
        stream::{SelectAll, StreamExt},
    };
    use maplit::hashmap;

    pub struct Device {
        keys: [signal::state_source::Signal<Option<Boolean>>; hardware::KEY_COUNT],
        leds: [signal::state_target::Signal<Boolean>; hardware::LED_COUNT],
        buzzer: signal::event_target::Signal<TimeDuration>,
        temperature: signal::state_source::Signal<Option<Temperature>>,

        sse_sender: waker_stream::Sender,
    }
    #[async_trait]
    impl logic::Device for Device {
        type HardwareDevice = hardware::Device;

        fn new() -> Self {
            let keys = array_init(|_| signal::state_source::Signal::new(None));
            let leds = array_init(|_| signal::state_target::Signal::new());
            let buzzer = signal::event_target::Signal::new();
            let temperature = signal::state_source::Signal::new(None);

            let sse_sender = waker_stream::Sender::new();

            Self {
                keys,
                leds,
                buzzer,
                temperature,

                sse_sender,
            }
        }
        fn class() -> &'static str {
            "junction_box_minimal_v1"
        }

        fn signals(&self) -> Signals {
            hashmap! {
                10 => &self.keys[0] as &dyn SignalBase,
                11 => &self.keys[1] as &dyn SignalBase,
                12 => &self.keys[2] as &dyn SignalBase,
                13 => &self.keys[3] as &dyn SignalBase,
                14 => &self.keys[4] as &dyn SignalBase,
                15 => &self.keys[5] as &dyn SignalBase,

                20 => &self.leds[0] as &dyn SignalBase,
                21 => &self.leds[1] as &dyn SignalBase,
                22 => &self.leds[2] as &dyn SignalBase,
                23 => &self.leds[3] as &dyn SignalBase,
                24 => &self.leds[4] as &dyn SignalBase,
                25 => &self.leds[5] as &dyn SignalBase,

                30 => &self.buzzer as &dyn SignalBase,

                40 => &self.temperature as &dyn SignalBase,
            }
        }

        async fn run(
            &self,
            remote_properties: hardware::RemoteProperties<'_>,
        ) -> ! {
            let hardware::RemoteProperties {
                keys,
                leds,
                buzzer,
                temperature,
            } = remote_properties;

            let keys_runner = keys.for_each(async move |key_values| {
                for (index, key) in self.keys.iter().enumerate() {
                    let key_value = key_values.map(|key_values| Boolean::from(key_values[index]));
                    key.set(key_value);
                }
            });
            pin_mut!(keys_runner);

            let leds_ref = &leds;
            let leds_runner = self
                .leds
                .iter()
                .map(|led| led.stream().map(|_| ()))
                .collect::<SelectAll<_>>()
                .map(|()| {
                    array_init(|index| {
                        self.leds[index]
                            .current()
                            .map(|value| value.into())
                            .unwrap_or(false)
                    })
                })
                .for_each(async move |value| leds_ref.set(value));
            pin_mut!(leds_runner);

            let buzzer_ref = &buzzer;
            let buzzer_runner = self
                .buzzer
                .stream()
                .map(|value| value.into())
                .for_each(async move |value| buzzer_ref.set(value));
            pin_mut!(buzzer_runner);

            let temperature_runner =
                temperature.for_each(async move |value| self.temperature.set(value));
            pin_mut!(temperature_runner);

            select! {
                () = keys_runner => panic!("keys_runner yielded"),
                () = leds_runner => panic!("leds_runner yielded"),
                () = buzzer_runner => panic!("buzzer_runner yielded"),
                () = temperature_runner => panic!("temperature_runner yielded"),
            }
        }
        async fn finalize(self) {}
    }
    impl Handler for Device {
        fn handle(
            &self,
            _request: Request,
            _uri_cursor: &UriCursor,
        ) -> BoxFuture<'static, Response> {
            async move { Response::error_404() }.boxed()
        }
    }
    impl NodeProvider for Device {
        fn node(&self) -> Node {
            Node::Terminal(self.sse_sender.receiver_factory())
        }
    }
}

pub mod hardware {
    use super::super::{
        super::houseblocks_v1::common::{AddressDeviceType, Payload},
        hardware::{
            common::ds18x20,
            driver::ApplicationDriver,
            parser::{Parser, ParserPayload},
            property, runner,
            serializer::Serializer,
        },
    };
    use crate::datatypes::temperature::Temperature;
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use failure::{err_msg, format_err, Error};
    use futures::{pin_mut, select, stream::StreamExt};
    use std::{
        cmp::{max, min},
        time::Duration,
    };

    pub const KEY_COUNT: usize = 6;
    pub type KeyValues = [bool; KEY_COUNT];

    pub const LED_COUNT: usize = 6;
    pub type LedValues = [bool; LED_COUNT];

    pub struct Device {
        keys: property::state_in::Property<KeyValues>,
        leds: property::state_out::Property<LedValues>,
        buzzer: property::event_last_out::Property<Duration>,
        temperature: property::state_in::Property<Temperature>,
    }
    pub struct RemoteProperties<'d> {
        pub keys: property::state_in::ValueStream<'d, KeyValues>,
        pub leds: property::state_out::ValueSink<'d, LedValues>,
        pub buzzer: property::event_last_out::ValueSink<'d, Duration>,
        pub temperature: property::state_in::ValueStream<'d, Temperature>,
    }
    #[async_trait]
    impl runner::Device for Device {
        fn new() -> Self {
            Self {
                keys: property::state_in::Property::new(),
                leds: property::state_out::Property::new([false; LED_COUNT]),
                buzzer: property::event_last_out::Property::new(),
                temperature: property::state_in::Property::new(),
            }
        }

        fn device_type_name() -> &'static str {
            "JunctionBox_Minimal_v1"
        }
        fn address_device_type() -> AddressDeviceType {
            AddressDeviceType::new_from_ordinal(3).unwrap()
        }

        type RemoteProperties<'d> = RemoteProperties<'d>;
        fn remote_properties(&self) -> RemoteProperties<'_> {
            RemoteProperties {
                keys: self.keys.user_get_stream(),
                leds: self.leds.user_get_sink(),
                buzzer: self.buzzer.user_get_sink(),
                temperature: self.temperature.user_get_stream(),
            }
        }

        async fn run(
            &self,
            run_context: &dyn runner::RunContext,
        ) -> ! {
            let leds_runner = self.leds.device_get_stream().for_each(async move |()| {
                run_context.poll_request();
            });
            pin_mut!(leds_runner);

            let buzzer_runner = self.buzzer.device_get_stream().for_each(async move |()| {
                run_context.poll_request();
            });
            pin_mut!(buzzer_runner);

            select! {
                () = leds_runner => panic!("leds_runner yielded"),
                () = buzzer_runner => panic!("buzzer_runner yielded"),
            }
        }
        async fn finalize(self) {}
    }
    #[async_trait]
    impl runner::BusDevice for Device {
        async fn initialize(
            &self,
            _driver: &dyn ApplicationDriver,
        ) -> Result<(), Error> {
            Ok(())
        }

        fn poll_delay(&self) -> Option<Duration> {
            Some(Duration::from_millis(250))
        }
        async fn poll(
            &self,
            driver: &dyn ApplicationDriver,
        ) -> Result<(), Error> {
            // Stage 1 - Poll + Pending values
            let leds_pending = self.leds.device_get_pending();
            let buzzer_pending = self.buzzer.device_get_pending();

            let stage_1_request = BusRequest::new(
                true,
                false,
                leds_pending
                    .as_ref()
                    .map(|leds_pending| BusRequestLeds::new(**leds_pending)),
                buzzer_pending.as_ref().map(|buzzer_pending| {
                    BusRequestBuzzer::from_duration_milliseconds(**buzzer_pending)
                }),
                false,
            );
            let stage_1_request_payload = stage_1_request.into_payload();
            let stage_1_response_payload = driver
                .transaction_out_in(stage_1_request_payload, None)
                .await?;
            let stage_1_response = BusResponse::from_payload(stage_1_response_payload)?;

            if let Some(leds_pending) = leds_pending {
                leds_pending.commit();
            }
            if let Some(buzzer_pending) = buzzer_pending {
                buzzer_pending.commit();
            }

            // Stage 2 - If poll returned something, handle it
            let stage_1_response_poll = match stage_1_response.poll {
                Some(stage_1_response_poll) => stage_1_response_poll,
                None => return Ok(()),
            };
            let stage_2_request = BusRequest::new(
                false,
                stage_1_response_poll.keys || !self.keys.device_is_set(),
                None,
                None,
                stage_1_response_poll.temperature,
            );
            let stage_2_request_payload = stage_2_request.into_payload();
            let stage_2_response_payload = driver
                .transaction_out_in(stage_2_request_payload, None)
                .await?;
            let stage_2_response = BusResponse::from_payload(stage_2_response_payload)?;

            if let Some(response_keys) = stage_2_response.keys {
                self.keys.device_set(response_keys.values());
            }
            if let Some(response_temperature) = stage_2_response.temperature {
                match response_temperature.as_temperature() {
                    Some(temperature) => self.temperature.device_set(temperature),
                    None => {
                        log::warn!(
                            "temperature sensor failure ({:?}, {})",
                            response_temperature.state.sensor_type(),
                            response_temperature.state.reset_count()
                        );
                        self.temperature.device_set_unknown();
                    }
                };
            }

            Ok(())
        }

        async fn deinitialize(
            &self,
            _driver: &dyn ApplicationDriver,
        ) -> Result<(), Error> {
            Ok(())
        }

        fn failed(&self) {
            self.keys.device_set_unknown();
            self.leds.device_set_unknown();
            self.temperature.device_set_unknown();
        }
    }

    // Bus
    #[derive(Debug)]
    struct BusRequestLeds {
        values: [bool; LED_COUNT],
    }
    impl BusRequestLeds {
        pub fn new(values: [bool; LED_COUNT]) -> Self {
            Self { values }
        }

        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            let mut values = ArrayVec::<[bool; 8]>::new();
            values.try_extend_from_slice(&self.values).unwrap();
            values.push(false);
            values.push(false);
            serializer.push_bool_array_8(values);
        }
    }

    #[derive(Debug)]
    struct BusRequestBuzzer {
        ticks: u8,
    }
    impl BusRequestBuzzer {
        pub fn new(ticks: u8) -> Self {
            Self { ticks }
        }
        pub fn from_duration_milliseconds(duration: Duration) -> Self {
            let ticks = max(
                min(
                    (duration.as_millis() as f64 / 5.0).ceil() as u64,
                    u8::MAX as u64,
                ) as u8,
                1u8,
            );
            Self::new(ticks)
        }

        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            serializer.push_u8(self.ticks);
        }
    }

    #[derive(Debug)]
    struct BusRequest {
        poll_request: bool,
        keys_request: bool,
        leds: Option<BusRequestLeds>,
        buzzer: Option<BusRequestBuzzer>,
        temperature_request: bool,
    }
    impl BusRequest {
        pub fn new(
            poll_request: bool,
            keys_request: bool,
            leds: Option<BusRequestLeds>,
            buzzer: Option<BusRequestBuzzer>,
            temperature_request: bool,
        ) -> Self {
            Self {
                poll_request,
                keys_request,
                leds,
                buzzer,
                temperature_request,
            }
        }

        pub fn into_payload(self) -> Payload {
            let mut serializer = Serializer::new();
            self.serialize(&mut serializer);
            serializer.into_payload()
        }

        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            if self.poll_request {
                serializer.push_byte(b'P');
            }
            if self.keys_request {
                serializer.push_byte(b'K');
            }
            if let Some(leds) = self.leds.as_ref() {
                serializer.push_byte(b'L');
                leds.serialize(serializer);
            }
            if let Some(buzzer) = self.buzzer.as_ref() {
                serializer.push_byte(b'B');
                buzzer.serialize(serializer);
            }
            if self.temperature_request {
                serializer.push_byte(b'T');
            }
        }
    }

    #[cfg(test)]
    mod test_bus_request {
        use super::{
            super::super::super::houseblocks_v1::common::Payload, BusRequest, BusRequestBuzzer,
            BusRequestLeds,
        };

        #[test]
        fn test_1() {
            let request = BusRequest {
                poll_request: false,
                keys_request: false,
                leds: None,
                buzzer: None,
                temperature_request: false,
            };
            let payload = request.into_payload();
            assert_eq!(payload, Payload::new(Box::from(*b"")).unwrap());
        }
        #[test]
        fn test_2() {
            let request = BusRequest {
                poll_request: true,
                keys_request: true,
                leds: Some(BusRequestLeds {
                    values: [true, false, false, true, true, true],
                }),
                buzzer: Some(BusRequestBuzzer { ticks: 0xF1 }),
                temperature_request: true,
            };
            let payload = request.into_payload();
            assert_eq!(payload, Payload::new(Box::from(*b"PKL39BF1T")).unwrap());
        }
    }

    #[derive(Debug)]
    struct BusResponsePoll {
        keys: bool,
        temperature: bool,
    }
    impl BusResponsePoll {
        pub fn parse(parser: &mut impl Parser) -> Result<Self, Error> {
            let keys = parser.expect_bool()?;
            let temperature = parser.expect_bool()?;
            Ok(Self { keys, temperature })
        }
    }

    #[derive(Debug)]
    struct BusResponseKey {
        value: bool,
        changes_count: u8,
    }
    impl BusResponseKey {
        pub fn parse(parser: &mut impl Parser) -> Result<Self, Error> {
            let value = parser.expect_bool()?;
            let changes_count = parser.expect_u8()?;
            Ok(Self {
                value,
                changes_count,
            })
        }

        pub fn value(&self) -> bool {
            self.value
        }
    }

    #[derive(Debug)]
    struct BusResponseKeys {
        keys: [BusResponseKey; KEY_COUNT],
    }
    impl BusResponseKeys {
        pub fn parse(parser: &mut impl Parser) -> Result<Self, Error> {
            let keys = (0..KEY_COUNT)
                .map(|_| BusResponseKey::parse(parser))
                .collect::<Result<ArrayVec<[_; KEY_COUNT]>, _>>()?
                .into_inner()
                .unwrap();
            Ok(Self { keys })
        }

        pub fn values(&self) -> KeyValues {
            self.keys
                .iter()
                .map(|key| key.value())
                .collect::<ArrayVec<[_; KEY_COUNT]>>()
                .into_inner()
                .unwrap()
        }
    }

    #[derive(Debug)]
    struct BusResponseTemperature {
        state: ds18x20::State,
    }
    impl BusResponseTemperature {
        pub fn parse(parser: &mut impl Parser) -> Result<Self, Error> {
            let state = ds18x20::State::parse(parser)?;
            Ok(Self { state })
        }

        pub fn as_temperature(&self) -> Option<Temperature> {
            self.state.temperature()
        }
    }

    #[derive(Debug)]
    struct BusResponse {
        poll: Option<BusResponsePoll>,
        keys: Option<BusResponseKeys>,
        temperature: Option<BusResponseTemperature>,
    }
    impl BusResponse {
        pub fn from_payload(payload: Payload) -> Result<Self, Error> {
            let mut parser = ParserPayload::new(&payload);
            let self_ = Self::parse(&mut parser)?;
            Ok(self_)
        }

        pub fn parse(parser: &mut impl Parser) -> Result<Self, Error> {
            let mut poll = None;
            let mut keys = None;
            let mut temperature = None;

            while let Some(opcode) = parser.get_byte() {
                match opcode {
                    b'P' => {
                        if poll.replace(BusResponsePoll::parse(parser)?).is_some() {
                            return Err(err_msg("duplicated poll"));
                        }
                    }
                    b'K' => {
                        if keys.replace(BusResponseKeys::parse(parser)?).is_some() {
                            return Err(err_msg("duplicated keys"));
                        }
                    }
                    b'T' => {
                        if temperature
                            .replace(BusResponseTemperature::parse(parser)?)
                            .is_some()
                        {
                            return Err(err_msg("duplicated temperature"));
                        }
                    }
                    opcode => return Err(format_err!("unrecognized opcode: {}", opcode)),
                }
            }

            Ok(Self {
                poll,
                keys,
                temperature,
            })
        }
    }

    #[cfg(test)]
    mod test_bus_response {
        use super::{super::super::super::houseblocks_v1::common::Payload, BusResponse};
        use crate::datatypes::temperature::{Temperature, Unit as TemperatureUnit};

        #[test]
        fn empty_1() {
            let payload = Payload::new(Box::from(*b"")).unwrap();
            let bus_response = BusResponse::from_payload(payload).unwrap();
            assert!(bus_response.poll.is_none());
            assert!(bus_response.keys.is_none());
            assert!(bus_response.temperature.is_none());
        }

        #[test]
        fn invalid_1() {
            let payload = Payload::new(Box::from(*b"1")).unwrap();
            BusResponse::from_payload(payload).unwrap_err();
        }
        #[test]
        fn invalid_2() {
            let payload = Payload::new(Box::from(*b"P00P11")).unwrap();
            BusResponse::from_payload(payload).unwrap_err();
        }

        #[test]
        fn response_1() {
            let payload = Payload::new(Box::from(*b"P01TC7D0")).unwrap();
            let bus_response = BusResponse::from_payload(payload).unwrap();
            assert_eq!(bus_response.poll.as_ref().unwrap().keys, false);
            assert_eq!(bus_response.poll.as_ref().unwrap().temperature, true);
            assert!(bus_response.keys.is_none());
            assert_eq!(
                bus_response.temperature.unwrap().as_temperature().unwrap(),
                Temperature::new(TemperatureUnit::CELSIUS, 125.00)
            );
        }
        #[test]
        fn response_2() {
            let payload = Payload::new(Box::from(*b"P10K0001FF0121230AA1EE")).unwrap();
            let bus_response = BusResponse::from_payload(payload).unwrap();
            assert_eq!(bus_response.poll.as_ref().unwrap().keys, true);
            assert_eq!(bus_response.poll.as_ref().unwrap().temperature, false);

            assert_eq!(bus_response.keys.as_ref().unwrap().keys[0].value, false);
            assert_eq!(bus_response.keys.as_ref().unwrap().keys[0].changes_count, 0);

            assert_eq!(bus_response.keys.as_ref().unwrap().keys[1].value, true);
            assert_eq!(
                bus_response.keys.as_ref().unwrap().keys[1].changes_count,
                0xFF
            );

            assert_eq!(bus_response.keys.as_ref().unwrap().keys[2].value, false);
            assert_eq!(
                bus_response.keys.as_ref().unwrap().keys[2].changes_count,
                0x12
            );

            assert_eq!(bus_response.keys.as_ref().unwrap().keys[3].value, true);
            assert_eq!(
                bus_response.keys.as_ref().unwrap().keys[3].changes_count,
                0x23
            );

            assert_eq!(bus_response.keys.as_ref().unwrap().keys[4].value, false);
            assert_eq!(
                bus_response.keys.as_ref().unwrap().keys[4].changes_count,
                0xAA
            );

            assert_eq!(bus_response.keys.as_ref().unwrap().keys[5].value, true);
            assert_eq!(
                bus_response.keys.as_ref().unwrap().keys[5].changes_count,
                0xEE
            );

            assert!(bus_response.temperature.is_none());
        }
        #[test]
        fn response_3() {
            let payload = Payload::new(Box::from(*b"P01")).unwrap();
            let bus_response = BusResponse::from_payload(payload).unwrap();
            assert_eq!(bus_response.poll.as_ref().unwrap().keys, false);
            assert_eq!(bus_response.poll.as_ref().unwrap().temperature, true);
            assert!(bus_response.keys.is_none());
            assert!(bus_response.temperature.is_none());
        }
    }
}
