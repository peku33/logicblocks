pub mod logic {
    use super::{super::logic, hardware};
    use crate::{
        datatypes::temperature::Temperature,
        devices,
        signals::{self, signal},
        util::{
            async_flag,
            runtime::{Exited, Runnable},
            waker_stream,
        },
    };
    use array_init::array_init;
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use maplit::hashmap;
    use serde::Serialize;
    use std::time::Duration;

    #[derive(Debug)]
    pub struct Device {
        properties_remote: hardware::PropertiesRemote,
        properties_remote_out_changed_waker: waker_stream::mpsc::SenderReceiver,

        signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
        signal_keys: [signal::state_source::Signal<bool>; hardware::KEY_COUNT],
        signal_leds: [signal::state_target_last::Signal<bool>; hardware::LED_COUNT],
        signal_buzzer: signal::event_target_last::Signal<Duration>,
        signal_temperature: signal::state_source::Signal<Temperature>,

        gui_summary_waker: waker_stream::mpmc::Sender,
    }

    impl logic::Device for Device {
        type HardwareDevice = hardware::Device;

        fn new(properties_remote: hardware::PropertiesRemote) -> Self {
            Self {
                properties_remote,
                properties_remote_out_changed_waker: waker_stream::mpsc::SenderReceiver::new(),

                signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
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
            if let Some(temperature) = self.properties_remote.temperature.take_pending() {
                let temperature = temperature
                    .map(|temperature| temperature.temperature())
                    .flatten();

                if self.signal_temperature.set_one(temperature) {
                    signals_changed = true;
                }
                gui_summary_changed = true;
            }

            if signals_changed {
                self.signal_sources_changed_waker.wake();
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
            exit_flag.await;
            Exited
        }
    }
    impl signals::Device for Device {
        fn signal_targets_changed_wake(&self) {
            let mut properties_remote_changed = false;

            // leds
            let leds_last = self
                .signal_leds
                .iter()
                .map(|signal_led| signal_led.take_last())
                .collect::<ArrayVec<_, { hardware::LED_COUNT }>>();
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
        fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
            self.signal_sources_changed_waker.receiver()
        }
        fn signals(&self) -> signals::Signals {
            hashmap! {
                10 => &self.signal_keys[0] as &dyn signal::Base,
                11 => &self.signal_keys[1] as &dyn signal::Base,
                12 => &self.signal_keys[2] as &dyn signal::Base,
                13 => &self.signal_keys[3] as &dyn signal::Base,
                14 => &self.signal_keys[4] as &dyn signal::Base,
                15 => &self.signal_keys[5] as &dyn signal::Base,

                20 => &self.signal_leds[0] as &dyn signal::Base,
                21 => &self.signal_leds[1] as &dyn signal::Base,
                22 => &self.signal_leds[2] as &dyn signal::Base,
                23 => &self.signal_leds[3] as &dyn signal::Base,
                24 => &self.signal_leds[4] as &dyn signal::Base,
                25 => &self.signal_leds[5] as &dyn signal::Base,

                30 => &self.signal_buzzer as &dyn signal::Base,

                40 => &self.signal_temperature as &dyn signal::Base,
            }
        }
    }

    #[derive(Serialize)]
    struct GuiSummary {
        temperature: Option<Temperature>,
    }
    impl devices::GuiSummaryProvider for Device {
        fn value(&self) -> Box<dyn devices::GuiSummary> {
            Box::new(GuiSummary {
                temperature: self
                    .properties_remote
                    .temperature
                    .get_last()
                    .map(|temperature| temperature.temperature())
                    .flatten(),
            })
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
            common::ds18x20,
            driver::ApplicationDriver,
            parser::{Parser, ParserPayload},
            property, runner,
            serializer::Serializer,
        },
    };
    use crate::util::{
        async_flag,
        runtime::{Exited, Runnable},
    };
    use anyhow::{bail, Context, Error};
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use maplit::hashmap;
    use std::{
        cmp::{max, min},
        collections::HashMap,
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
        temperature: property::state_in::Property<ds18x20::State>,
    }
    impl Properties {
        pub fn new() -> Self {
            Self {
                keys: property::state_event_in::Property::new(),
                leds: property::state_out::Property::new([false; LED_COUNT]),
                buzzer: property::event_out_last::Property::new(),
                temperature: property::state_in::Property::new(),
            }
        }
    }
    impl runner::Properties for Properties {
        fn by_name(&self) -> HashMap<&'static str, &dyn property::Base> {
            hashmap! {
                "keys" => &self.keys as &dyn property::Base,
                "leds" => &self.leds as &dyn property::Base,
                "buzzer" => &self.buzzer as &dyn property::Base,
                "temperature" => &self.temperature as &dyn property::Base,
            }
        }

        fn in_any_user_pending(&self) -> bool {
            self.keys.user_pending() || self.temperature.user_pending()
        }

        type Remote = PropertiesRemote;
        fn remote(&self) -> Self::Remote {
            PropertiesRemote {
                keys: self.keys.user_stream(),
                leds: self.leds.user_sink(),
                buzzer: self.buzzer.user_sink(),
                temperature: self.temperature.user_stream(),
            }
        }
    }
    #[derive(Debug)]
    pub struct PropertiesRemote {
        pub keys: property::state_event_in::Stream<KeyValues, KeyChangesCount>,
        pub leds: property::state_out::Sink<LedValues>,
        pub buzzer: property::event_out_last::Sink<Duration>,
        pub temperature: property::state_in::Stream<ds18x20::State>,
    }

    #[derive(Debug)]
    pub struct Device {
        properties: Properties,
    }
    impl runner::Device for Device {
        fn new() -> Self {
            Self {
                properties: Properties::new(),
            }
        }

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
    impl Runnable for Device {
        async fn run(
            &self,
            exit_flag: async_flag::Receiver,
        ) -> Exited {
            exit_flag.await;
            Exited
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
                .await
                .context("stage 1 transaction")?;
            let stage_1_response =
                BusResponse::from_payload(stage_1_response_payload).context("stage 1 response")?;

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
                stage_1_response_poll.keys || self.properties.keys.device_must_read(),
                None,
                None,
                stage_1_response_poll.temperature || self.properties.temperature.device_must_read(),
            );
            let stage_2_request_payload = stage_2_request.into_payload();
            let stage_2_response_payload = driver
                .transaction_out_in(stage_2_request_payload, None)
                .await
                .context("stage 2 transaction")?;
            let stage_2_response =
                BusResponse::from_payload(stage_2_response_payload).context("stage 2 response")?;

            // Propagate values to properties
            if let Some(response_keys) = stage_2_response.keys {
                self.properties
                    .keys
                    .device_set(response_keys.values(), response_keys.changes_count());
            }

            if let Some(response_temperature) = stage_2_response.temperature {
                self.properties
                    .temperature
                    .device_set(response_temperature.state());
            }

            Ok(())
        }

        async fn deinitialize(
            &self,
            _driver: &ApplicationDriver<'_>,
        ) -> Result<(), Error> {
            Ok(())
        }

        fn failed(&self) {
            self.properties.keys.device_reset();
            self.properties.leds.device_reset();
            self.properties.temperature.device_reset();
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
            let mut values = ArrayVec::<bool, 8>::new();
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
    mod tests_bus_request {
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
            let keys = parser.expect_bool().context("keys")?;
            let temperature = parser.expect_bool().context("temperature")?;
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
            let value = parser.expect_bool().context("value")?;
            let changes_count = parser.expect_u8().context("changes_count")?;
            Ok(Self {
                value,
                changes_count,
            })
        }

        pub fn value(&self) -> bool {
            self.value
        }
        pub fn changes_count(&self) -> u8 {
            self.changes_count
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
                .collect::<Result<ArrayVec<_, { KEY_COUNT }>, _>>()?
                .into_inner()
                .unwrap();
            Ok(Self { keys })
        }

        pub fn values(&self) -> KeyValues {
            self.keys
                .iter()
                .map(|key| key.value())
                .collect::<ArrayVec<_, { KEY_COUNT }>>()
                .into_inner()
                .unwrap()
        }
        pub fn changes_count(&self) -> KeyChangesCount {
            self.keys
                .iter()
                .map(|key| key.changes_count())
                .collect::<ArrayVec<_, { KEY_COUNT }>>()
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
            let state = ds18x20::State::parse(parser).context("state")?;
            Ok(Self { state })
        }

        pub fn state(&self) -> ds18x20::State {
            self.state
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
                        if poll
                            .replace(BusResponsePoll::parse(parser).context("poll")?)
                            .is_some()
                        {
                            bail!("duplicated poll");
                        }
                    }
                    b'K' => {
                        if keys
                            .replace(BusResponseKeys::parse(parser).context("keys")?)
                            .is_some()
                        {
                            bail!("duplicated keys");
                        }
                    }
                    b'T' => {
                        if temperature
                            .replace(BusResponseTemperature::parse(parser).context("temperature")?)
                            .is_some()
                        {
                            bail!("duplicated temperature");
                        }
                    }
                    opcode => bail!("unrecognized opcode: {}", opcode),
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
    mod tests_bus_response {
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
                bus_response
                    .temperature
                    .unwrap()
                    .state()
                    .temperature()
                    .unwrap(),
                Temperature::new(TemperatureUnit::Celsius, 125.00)
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
