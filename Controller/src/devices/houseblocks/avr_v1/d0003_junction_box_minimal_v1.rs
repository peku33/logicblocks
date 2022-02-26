pub mod logic {
    use super::{super::logic, hardware};
    use crate::{
        datatypes::temperature::Temperature,
        devices,
        signals::{self, signal},
        util::{
            async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
            async_flag,
            runtime::{Exited, Runnable},
            waker_stream,
        },
    };
    use array_init::array_init;
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use futures::stream::StreamExt;
    use serde::Serialize;
    use std::{iter, time::Duration};

    #[derive(Debug)]
    pub struct Device {
        properties_remote: hardware::PropertiesRemote,
        properties_remote_out_changed_waker: waker_stream::mpsc::SenderReceiver,

        signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
        signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
        signal_keys: [signal::state_source::Signal<bool>; hardware::KEY_COUNT],
        signal_leds: [signal::state_target_last::Signal<bool>; hardware::LED_COUNT],
        signal_buzzer: signal::event_target_last::Signal<Duration>,
        signal_temperature: signal::state_source::Signal<Temperature>,

        gui_summary_waker: waker_stream::mpmc::Sender,
    }

    impl Device {
        fn signals_targets_changed(&self) {
            let mut properties_remote_changed = false;

            // leds
            let leds_last = self
                .signal_leds
                .iter()
                .map(|signal_led| signal_led.take_last())
                .collect::<ArrayVec<_, { hardware::LED_COUNT }>>()
                .into_inner()
                .unwrap();
            if leds_last.iter().any(|led_last| led_last.pending) {
                let leds = leds_last
                    .iter()
                    .map(|led_last| led_last.value.unwrap_or(false))
                    .collect::<ArrayVec<_, { hardware::LED_COUNT }>>()
                    .into_inner()
                    .unwrap();

                if self.properties_remote.leds.set(leds) {
                    properties_remote_changed = true;
                }
            }

            // buzzer
            if let Some(buzzer) = self.signal_buzzer.take_pending() {
                if self.properties_remote.buzzer.push(buzzer) {
                    properties_remote_changed = true;
                }
            }

            if properties_remote_changed {
                self.properties_remote_out_changed_waker.wake();
            }
        }

        async fn run(
            &self,
            exit_flag: async_flag::Receiver,
        ) -> Exited {
            self.signals_targets_changed_waker
                .stream(false)
                .stream_take_until_exhausted(exit_flag)
                .for_each(async move |()| {
                    self.signals_targets_changed();
                })
                .await;

            Exited
        }
    }

    impl logic::Device for Device {
        type HardwareDevice = hardware::Device;

        fn new(properties_remote: hardware::PropertiesRemote) -> Self {
            Self {
                properties_remote,
                properties_remote_out_changed_waker: waker_stream::mpsc::SenderReceiver::new(),

                signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
                signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
                signal_keys: array_init(|_| signal::state_source::Signal::<bool>::new(None)),
                signal_leds: array_init(|_| signal::state_target_last::Signal::<bool>::new()),
                signal_buzzer: signal::event_target_last::Signal::<Duration>::new(),
                signal_temperature: signal::state_source::Signal::<Temperature>::new(None),

                gui_summary_waker: waker_stream::mpmc::Sender::new(),
            }
        }
        fn class() -> &'static str {
            "junction_box_minimal_v1"
        }

        fn as_runnable(&self) -> &dyn Runnable {
            self
        }
        fn as_gui_summary_provider(&self) -> Option<&dyn devices::GuiSummaryProvider> {
            Some(self)
        }

        fn properties_remote_in_changed(&self) {
            let mut signals_changed = false;
            let mut gui_summary_changed = false;

            // keys
            if let Some((key_values, key_changes_count_queue)) =
                self.properties_remote.keys.take_pending()
            {
                if let Some(key_values) = key_values {
                    // Calculate total number of key ticks
                    let mut key_changes_count_merged = [0usize; hardware::KEY_COUNT];
                    for key_changes_count in key_changes_count_queue.into_vec().into_iter() {
                        for (key_index, key_changes_count) in key_changes_count.iter().enumerate() {
                            key_changes_count_merged[key_index] += *key_changes_count as usize;
                        }
                    }

                    // Set total number of key ticks
                    self.signal_keys
                        .iter()
                        .enumerate()
                        .for_each(|(key_index, signal_key)| {
                            let key_values = (0..key_changes_count_merged[key_index])
                                .rev()
                                .map(|key_change_index| {
                                    let key_value =
                                        key_values[key_index] ^ (key_change_index % 2 != 0);
                                    Some(key_value)
                                })
                                .collect::<Box<[_]>>();
                            if signal_key.set_many(key_values) {
                                signals_changed = true;
                            }
                        });
                } else {
                    // Keys are broken
                    self.signal_keys.iter().for_each(|signal_key| {
                        if signal_key.set_one(None) {
                            signals_changed = true;
                        }
                    });
                }
            }

            // temperature
            if let Some(ds18x20) = self.properties_remote.ds18x20.take_pending() {
                let temperature = ds18x20.and_then(|ds18x20| ds18x20.temperature);

                if self.signal_temperature.set_one(temperature) {
                    signals_changed = true;
                }
                gui_summary_changed = true;
            }

            if signals_changed {
                self.signals_sources_changed_waker.wake();
            }
            if gui_summary_changed {
                self.gui_summary_waker.wake();
            }
        }
        fn properties_remote_out_changed_waker_receiver(
            &self
        ) -> waker_stream::mpsc::ReceiverLease {
            self.properties_remote_out_changed_waker.receiver()
        }
    }

    #[async_trait]
    impl Runnable for Device {
        async fn run(
            &self,
            exit_flag: async_flag::Receiver,
        ) -> Exited {
            self.run(exit_flag).await
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub enum SignalIdentifier {
        Key(usize),
        Led(usize),
        Buzzer,
        Temperature,
    }
    impl signals::Identifier for SignalIdentifier {}
    impl signals::Device for Device {
        fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
            Some(&self.signals_targets_changed_waker)
        }
        fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
            Some(&self.signals_sources_changed_waker)
        }

        type Identifier = SignalIdentifier;
        fn by_identifier(&self) -> signals::ByIdentifier<Self::Identifier> {
            iter::empty()
                .chain(
                    self.signal_keys
                        .iter()
                        .enumerate()
                        .map(|(key_index, signal_key)| {
                            (
                                SignalIdentifier::Key(key_index),
                                signal_key as &dyn signal::Base,
                            )
                        }),
                )
                .chain(
                    self.signal_leds
                        .iter()
                        .enumerate()
                        .map(|(led_index, signal_led)| {
                            (
                                SignalIdentifier::Led(led_index),
                                signal_led as &dyn signal::Base,
                            )
                        }),
                )
                .chain([
                    (
                        SignalIdentifier::Buzzer,
                        &self.signal_buzzer as &dyn signal::Base,
                    ),
                    (
                        SignalIdentifier::Temperature,
                        &self.signal_temperature as &dyn signal::Base,
                    ),
                ])
                .collect()
        }
    }

    #[derive(Serialize)]
    struct GuiSummary {
        temperature: Option<Temperature>,
    }
    impl devices::GuiSummaryProvider for Device {
        fn value(&self) -> Box<dyn devices::GuiSummary> {
            let gui_summary = GuiSummary {
                temperature: self
                    .properties_remote
                    .ds18x20
                    .peek_last()
                    .and_then(|ds18x20| ds18x20.temperature),
            };
            let gui_summary = Box::new(gui_summary);
            gui_summary
        }

        fn waker(&self) -> waker_stream::mpmc::ReceiverFactory {
            self.gui_summary_waker.receiver_factory()
        }
    }
}

pub mod hardware {
    use super::super::{
        super::houseblocks_v1::common::{AddressDeviceType, Payload},
        hardware::{
            common::ds18x20, driver::ApplicationDriver, parser::Parser, property, runner,
            serializer::Serializer,
        },
    };
    use anyhow::{bail, ensure, Context, Error};
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use std::{
        cmp::{max, min},
        iter,
        time::Duration,
    };

    pub const KEY_COUNT: usize = 6;
    pub type KeyValues = [bool; KEY_COUNT];
    pub type KeyChangesCount = [u8; KEY_COUNT];

    pub const LED_COUNT: usize = 6;
    pub type LedValues = [bool; LED_COUNT];

    #[derive(Debug)]
    pub struct Properties {
        keys: property::state_event_in::Property<KeyValues, KeyChangesCount>,
        leds: property::state_out::Property<LedValues>,
        buzzer: property::event_out_last::Property<Duration>,
        ds18x20: property::state_in::Property<ds18x20::State>,
    }
    impl Properties {
        pub fn new() -> Self {
            Self {
                keys: property::state_event_in::Property::<KeyValues, KeyChangesCount>::new(),
                leds: property::state_out::Property::<LedValues>::new([false; LED_COUNT]),
                buzzer: property::event_out_last::Property::<Duration>::new(),
                ds18x20: property::state_in::Property::<ds18x20::State>::new(),
            }
        }
    }
    impl runner::Properties for Properties {
        fn user_pending(&self) -> bool {
            self.keys.user_pending() || self.ds18x20.user_pending()
        }
        fn device_reset(&self) {
            self.keys.device_reset();
            self.leds.device_reset();
            self.ds18x20.device_reset();
        }

        type Remote = PropertiesRemote;
        fn remote(&self) -> Self::Remote {
            PropertiesRemote {
                keys: self.keys.user_stream(),
                leds: self.leds.user_sink(),
                buzzer: self.buzzer.user_sink(),
                ds18x20: self.ds18x20.user_stream(),
            }
        }
    }
    #[derive(Debug)]
    pub struct PropertiesRemote {
        pub keys: property::state_event_in::Stream<KeyValues, KeyChangesCount>,
        pub leds: property::state_out::Sink<LedValues>,
        pub buzzer: property::event_out_last::Sink<Duration>,
        pub ds18x20: property::state_in::Stream<ds18x20::State>,
    }

    #[derive(Debug)]
    pub struct Device {
        properties: Properties,
    }
    impl Device {
        pub fn new() -> Self {
            Self {
                properties: Properties::new(),
            }
        }
    }
    impl runner::Device for Device {
        fn device_type_name() -> &'static str {
            "JunctionBox_Minimal_v1"
        }
        fn address_device_type() -> AddressDeviceType {
            AddressDeviceType::new_from_ordinal(3).unwrap()
        }

        type Properties = Properties;
        fn properties(&self) -> &Self::Properties {
            &self.properties
        }
    }
    #[async_trait]
    impl runner::BusDevice for Device {
        async fn initialize(
            &'_ self,
            _driver: &ApplicationDriver<'_>,
        ) -> Result<(), Error> {
            Ok(())
        }

        fn poll_delay(&self) -> Option<Duration> {
            Some(Duration::from_millis(250))
        }
        async fn poll(
            &self,
            driver: &ApplicationDriver<'_>,
        ) -> Result<(), Error> {
            // Stage 1 - Poll + Pending values
            let leds_pending = self.properties.leds.device_pending();
            let buzzer_pending = self.properties.buzzer.device_pending();

            let stage_1_request = BusRequest {
                poll_request: true,
                keys_request: false,
                leds: leds_pending.as_ref().map(|leds_pending| BusRequestLeds {
                    values: **leds_pending,
                }),
                buzzer: buzzer_pending.as_ref().map(|buzzer_pending| {
                    BusRequestBuzzer::from_duration_milliseconds(**buzzer_pending)
                }),
                ds18x20_request: false,
            };
            // stage 1 request is always used, to check device uptime
            let stage_1_request_payload = stage_1_request.to_payload();
            let stage_1_response_payload = driver
                .transaction_out_in(stage_1_request_payload, None)
                .await
                .context("stage 1 transaction")?;
            let stage_1_response =
                BusResponse::from_payload(&stage_1_response_payload).context("stage 1 response")?;

            if let Some(leds_pending) = leds_pending {
                leds_pending.commit()
            }
            if let Some(buzzer_pending) = buzzer_pending {
                buzzer_pending.commit()
            }

            let BusResponse {
                poll: stage_1_response_poll,
                keys: stage_1_response_keys,
                ds18x20: stage_1_response_ds18x20,
            } = stage_1_response;

            let stage_1_response_poll = match stage_1_response_poll {
                Some(stage_1_response_poll) => stage_1_response_poll,
                None => bail!("poll not returned after being requested"),
            };
            ensure!(stage_1_response_keys.is_none());
            ensure!(stage_1_response_ds18x20.is_none());

            // Stage 2 - If poll returned something, handle it
            let stage_2_request = BusRequest {
                poll_request: false,
                keys_request: false
                    || stage_1_response_poll.keys
                    || self.properties.keys.device_must_read(),
                leds: None,
                buzzer: None,
                ds18x20_request: false
                    || stage_1_response_poll.ds18x20
                    || self.properties.ds18x20.device_must_read(),
            };

            if stage_2_request.is_nop() {
                return Ok(());
            }

            let stage_2_request_payload = stage_2_request.to_payload();
            let stage_2_response_payload = driver
                .transaction_out_in(stage_2_request_payload, None)
                .await
                .context("stage 2 transaction")?;
            let stage_2_response =
                BusResponse::from_payload(&stage_2_response_payload).context("stage 2 response")?;

            let BusResponse {
                poll: stage_2_response_poll,
                keys: stage_2_response_keys,
                ds18x20: stage_2_response_ds18x20,
            } = stage_2_response;

            ensure!(stage_2_response_poll.is_none());
            let stage_2_response_keys = match (stage_2_request.keys_request, stage_2_response_keys)
            {
                (false, None) => None,
                (true, Some(stage_2_response_keys)) => Some(stage_2_response_keys),
                _ => bail!("keys mismatch"),
            };
            let stage_2_response_ds18x20 =
                match (stage_2_request.ds18x20_request, stage_2_response_ds18x20) {
                    (false, None) => None,
                    (true, Some(stage_2_response_ds18x20)) => Some(stage_2_response_ds18x20),
                    _ => bail!("ds18x20 mismatch"),
                };

            if let Some(stage_2_response_keys) = stage_2_response_keys {
                self.properties.keys.device_set(
                    stage_2_response_keys.values(),
                    stage_2_response_keys.changes_count(),
                );
            }
            if let Some(stage_2_response_ds18x20) = stage_2_response_ds18x20 {
                self.properties
                    .ds18x20
                    .device_set(stage_2_response_ds18x20.state);
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

    // Bus
    #[derive(PartialEq, Eq, Debug)]
    struct BusRequestLeds {
        pub values: [bool; LED_COUNT],
    }
    impl BusRequestLeds {
        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            let values = iter::empty()
                .chain(self.values.iter().copied())
                .chain(iter::repeat(false))
                .take(8)
                .collect::<ArrayVec<bool, 8>>()
                .into_inner()
                .unwrap();
            serializer.push_bool_array_8(values);
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusRequestBuzzer {
        ticks: u8,
    }
    impl BusRequestBuzzer {
        pub fn from_duration_milliseconds(duration: Duration) -> Self {
            let ticks = max(
                min(
                    (duration.as_millis() as f64 / 5.0).ceil() as u64,
                    u8::MAX as u64,
                ) as u8,
                1u8,
            );
            Self { ticks }
        }

        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            serializer.push_u8(self.ticks);
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusRequest {
        pub poll_request: bool,
        pub keys_request: bool,
        pub leds: Option<BusRequestLeds>,
        pub buzzer: Option<BusRequestBuzzer>,
        pub ds18x20_request: bool,
    }
    impl BusRequest {
        pub fn is_nop(&self) -> bool {
            *self
                == (Self {
                    poll_request: false,
                    keys_request: false,
                    leds: None,
                    buzzer: None,
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
            if self.ds18x20_request {
                serializer.push_byte(b'T');
            }
        }
    }
    #[cfg(test)]
    mod tests_bus_request {
        use super::{
            super::super::super::houseblocks_v1::common::Payload, BusRequest, BusRequestBuzzer,
            BusRequestLeds,
        };

        #[test]
        fn empty() {
            let request = BusRequest {
                poll_request: false,
                keys_request: false,
                leds: None,
                buzzer: None,
                ds18x20_request: false,
            };
            let payload = request.to_payload();

            let payload_expected = Payload::new(Box::from(*b"")).unwrap();

            assert_eq!(payload, payload_expected);
        }

        #[test]
        fn full() {
            let request = BusRequest {
                poll_request: true,
                keys_request: true,
                leds: Some(BusRequestLeds {
                    values: [true, false, false, true, true, true],
                }),
                buzzer: Some(BusRequestBuzzer { ticks: 0xF1 }),
                ds18x20_request: true,
            };
            let payload = request.to_payload();

            let payload_expected = Payload::new(Box::from(*b"PKL39BF1T")).unwrap();

            assert_eq!(payload, payload_expected);
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponsePoll {
        pub keys: bool,
        pub ds18x20: bool,
    }
    impl BusResponsePoll {
        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let keys = parser.expect_bool().context("keys")?;
            let ds18x20 = parser.expect_bool().context("ds18x20")?;
            Ok(Self { keys, ds18x20 })
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponseKey {
        pub value: bool,
        pub changes_count: u8,
    }
    impl BusResponseKey {
        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let value = parser.expect_bool().context("value")?;
            let changes_count = parser.expect_u8().context("changes_count")?;
            Ok(Self {
                value,
                changes_count,
            })
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponseKeys {
        pub keys: [BusResponseKey; KEY_COUNT],
    }
    impl BusResponseKeys {
        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let keys = (0..KEY_COUNT)
                .map(|_| BusResponseKey::parse(parser))
                .collect::<Result<ArrayVec<_, { KEY_COUNT }>, _>>()
                .context("collect")?
                .into_inner()
                .unwrap();
            Ok(Self { keys })
        }

        pub fn values(&self) -> KeyValues {
            self.keys
                .iter()
                .map(|key| key.value)
                .collect::<ArrayVec<_, { KEY_COUNT }>>()
                .into_inner()
                .unwrap()
        }
        pub fn changes_count(&self) -> KeyChangesCount {
            self.keys
                .iter()
                .map(|key| key.changes_count)
                .collect::<ArrayVec<_, { KEY_COUNT }>>()
                .into_inner()
                .unwrap()
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponseDs18x20 {
        pub state: ds18x20::State,
    }
    impl BusResponseDs18x20 {
        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let state = ds18x20::State::parse(parser).context("state")?;
            Ok(Self { state })
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponse {
        pub poll: Option<BusResponsePoll>,
        pub keys: Option<BusResponseKeys>,
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
            let mut keys: Option<BusResponseKeys> = None;
            let mut ds18x20: Option<BusResponseDs18x20> = None;

            while let Some(opcode) = parser.get_byte() {
                match opcode {
                    b'P' => {
                        let value = BusResponsePoll::parse(parser).context("poll")?;
                        ensure!(poll.replace(value).is_none(), "duplicated poll");
                    }
                    b'K' => {
                        let value = BusResponseKeys::parse(parser).context("keys")?;
                        ensure!(keys.replace(value).is_none(), "duplicated keys");
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
                keys,
                ds18x20,
            })
        }
    }
    #[cfg(test)]
    mod tests_bus_response {
        use super::{
            super::super::super::houseblocks_v1::common::Payload, BusResponse, BusResponseDs18x20,
            BusResponseKey, BusResponseKeys, BusResponsePoll,
        };
        use crate::{
            datatypes::temperature::{Temperature, Unit as TemperatureUnit},
            devices::houseblocks::avr_v1::hardware::common::ds18x20,
        };

        #[test]
        fn empty() {
            let payload = Payload::new(Box::from(*b"")).unwrap();
            let bus_response = BusResponse::from_payload(&payload).unwrap();

            let bus_response_expected = BusResponse {
                poll: None,
                keys: None,
                ds18x20: None,
            };

            assert_eq!(bus_response, bus_response_expected);
        }

        #[test]
        fn invalid_1() {
            let payload = Payload::new(Box::from(*b"1")).unwrap();
            BusResponse::from_payload(&payload).unwrap_err();
        }
        #[test]
        fn invalid_2() {
            let payload = Payload::new(Box::from(*b"P00P11")).unwrap();
            BusResponse::from_payload(&payload).unwrap_err();
        }

        #[test]
        fn response_1() {
            let payload = Payload::new(Box::from(*b"P01TC7D0")).unwrap();
            let bus_response = BusResponse::from_payload(&payload).unwrap();

            let bus_response_expected = BusResponse {
                poll: Some(BusResponsePoll {
                    keys: false,
                    ds18x20: true,
                }),
                keys: None,
                ds18x20: Some(BusResponseDs18x20 {
                    state: ds18x20::State {
                        sensor_type: ds18x20::SensorType::B,
                        reset_count: 0,
                        temperature: Some(Temperature::new(TemperatureUnit::Celsius, 125.00)),
                    },
                }),
            };

            assert_eq!(bus_response, bus_response_expected);
        }
        #[test]
        fn response_2() {
            let payload = Payload::new(Box::from(*b"P10K0001FF0121230AA1EE")).unwrap();
            let bus_response = BusResponse::from_payload(&payload).unwrap();

            let bus_response_expected = BusResponse {
                poll: Some(BusResponsePoll {
                    keys: true,
                    ds18x20: false,
                }),
                keys: Some(BusResponseKeys {
                    keys: [
                        BusResponseKey {
                            value: false,
                            changes_count: 0,
                        },
                        BusResponseKey {
                            value: true,
                            changes_count: 0xFF,
                        },
                        BusResponseKey {
                            value: false,
                            changes_count: 0x12,
                        },
                        BusResponseKey {
                            value: true,
                            changes_count: 0x23,
                        },
                        BusResponseKey {
                            value: false,
                            changes_count: 0xAA,
                        },
                        BusResponseKey {
                            value: true,
                            changes_count: 0xEE,
                        },
                    ],
                }),
                ds18x20: None,
            };

            assert_eq!(bus_response, bus_response_expected);
        }
        #[test]
        fn response_3() {
            let payload = Payload::new(Box::from(*b"P01")).unwrap();
            let bus_response = BusResponse::from_payload(&payload).unwrap();

            let bus_response_expected = BusResponse {
                poll: Some(BusResponsePoll {
                    keys: false,
                    ds18x20: true,
                }),
                keys: None,
                ds18x20: None,
            };

            assert_eq!(bus_response, bus_response_expected);
        }
    }
}
