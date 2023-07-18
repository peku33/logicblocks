pub mod logic {
    use super::{super::logic::runner, hardware};
    use crate::{
        datatypes::temperature::Temperature,
        devices,
        signals::{self, signal},
        util::{
            async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
            async_flag,
            runnable::{Exited, Runnable},
        },
    };
    use array_init::array_init;
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use futures::{future::FutureExt, join, stream::StreamExt};
    use serde::Serialize;
    use std::{iter, time::Duration};

    #[derive(Debug)]
    pub struct DeviceFactory;
    impl runner::DeviceFactory for DeviceFactory {
        type Device<'h> = Device<'h>;

        fn new(hardware_device: &hardware::Device) -> Device {
            Device::new(hardware_device)
        }
    }

    #[derive(Debug)]
    pub struct Device<'h> {
        properties_remote: hardware::PropertiesRemote<'h>,

        signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
        signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
        signal_keys: [signal::state_source::Signal<bool>; hardware::KEY_COUNT],
        signal_leds: [signal::state_target_last::Signal<bool>; hardware::LED_COUNT],
        signal_buzzer: signal::event_target_last::Signal<Duration>,
        signal_temperature: signal::state_source::Signal<Temperature>,

        gui_summary_waker: devices::gui_summary::Waker,
    }

    impl<'h> Device<'h> {
        pub fn new(hardware_device: &'h hardware::Device) -> Self {
            Self {
                properties_remote: hardware_device.properties_remote(),

                signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
                signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
                signal_keys: array_init(|_| signal::state_source::Signal::<bool>::new(None)),
                signal_leds: array_init(|_| signal::state_target_last::Signal::<bool>::new()),
                signal_buzzer: signal::event_target_last::Signal::<Duration>::new(),
                signal_temperature: signal::state_source::Signal::<Temperature>::new(None),

                gui_summary_waker: devices::gui_summary::Waker::new(),
            }
        }

        fn signals_targets_changed(&self) {
            let mut properties_outs_changed = false;

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
                    properties_outs_changed = true;
                }
            }

            // buzzer
            if let Some(buzzer) = self.signal_buzzer.take_pending() {
                if self.properties_remote.buzzer.push(buzzer) {
                    properties_outs_changed = true;
                }
            }

            if properties_outs_changed {
                self.properties_remote.outs_changed_waker_remote.wake();
            }
        }
        fn properties_ins_changed(&self) {
            let mut signals_sources_changed = false;
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
                                signals_sources_changed = true;
                            }
                        });
                } else {
                    // Keys are broken
                    self.signal_keys.iter().for_each(|signal_key| {
                        if signal_key.set_one(None) {
                            signals_sources_changed = true;
                        }
                    });
                }
            }

            // temperature
            if let Some(ds18x20) = self.properties_remote.ds18x20.take_pending() {
                let temperature = ds18x20.and_then(|ds18x20| ds18x20.temperature);

                if self.signal_temperature.set_one(temperature) {
                    signals_sources_changed = true;
                }

                gui_summary_changed = true;
            }

            if signals_sources_changed {
                self.signals_sources_changed_waker.wake();
            }
            if gui_summary_changed {
                self.gui_summary_waker.wake();
            }
        }

        async fn run(
            &self,
            exit_flag: async_flag::Receiver,
        ) -> Exited {
            // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
            let signals_targets_changed_runner = self
                .signals_targets_changed_waker
                .stream()
                .stream_take_until_exhausted(exit_flag.clone())
                .for_each(async move |()| {
                    self.signals_targets_changed();
                })
                .boxed();

            // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
            let properties_ins_changed_runner = self
                .properties_remote
                .ins_changed_waker_remote
                .stream()
                .stream_take_until_exhausted(exit_flag.clone())
                .for_each(async move |()| {
                    self.properties_ins_changed();
                })
                .boxed();

            let _: ((), ()) = join!(
                signals_targets_changed_runner,
                properties_ins_changed_runner
            );

            Exited
        }
    }

    impl<'h> runner::Device for Device<'h> {
        type HardwareDevice = hardware::Device;

        fn class() -> &'static str {
            "junction_box_minimal_v1"
        }

        fn as_runnable(&self) -> &dyn Runnable {
            self
        }
        fn as_gui_summary_device_base(&self) -> Option<&dyn devices::gui_summary::DeviceBase> {
            Some(self)
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
    impl<'h> signals::Device for Device<'h> {
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
                .collect::<signals::ByIdentifier<_>>()
        }
    }

    #[async_trait]
    impl<'h> Runnable for Device<'h> {
        async fn run(
            &self,
            exit_flag: async_flag::Receiver,
        ) -> Exited {
            self.run(exit_flag).await
        }
    }

    #[derive(Debug, Serialize)]
    pub struct GuiSummary {
        temperature: Option<Temperature>,
    }
    impl<'h> devices::gui_summary::Device for Device<'h> {
        fn waker(&self) -> &devices::gui_summary::Waker {
            &self.gui_summary_waker
        }

        type Value = GuiSummary;
        fn value(&self) -> Self::Value {
            let temperature = self
                .properties_remote
                .ds18x20
                .peek_last()
                .and_then(|ds18x20| ds18x20.temperature);

            Self::Value { temperature }
        }
    }
}

pub mod hardware {
    use super::super::{
        super::houseblocks_v1::common::{AddressDeviceType, Payload},
        datatypes::ds18x20::State as Ds18x20State,
        hardware::{
            datatypes::ds18x20::SensorState as Ds18x20SensorState, driver::ApplicationDriver,
            parser::Parser, runner, serializer::Serializer,
        },
        properties,
    };
    use crate::util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
        waker_stream,
    };
    use anyhow::{bail, ensure, Context, Error};
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use futures::{future::FutureExt, join, stream::StreamExt};
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
    pub struct PropertiesRemote<'p> {
        pub ins_changed_waker_remote: properties::waker::InsChangedWakerRemote<'p>,
        pub outs_changed_waker_remote: properties::waker::OutsChangedWakerRemote<'p>,

        pub keys: properties::state_event_in::Remote<'p, KeyValues, KeyChangesCount>,
        pub leds: properties::state_out::Remote<'p, LedValues>,
        pub buzzer: properties::event_out_last::Remote<'p, Duration>,
        pub ds18x20: properties::state_in::Remote<'p, Ds18x20State>,
    }

    #[derive(Debug)]
    pub struct Properties {
        ins_changed_waker: properties::waker::InsChangedWaker,
        outs_changed_waker: properties::waker::OutsChangedWaker,

        keys: properties::state_event_in::Property<KeyValues, KeyChangesCount>,
        leds: properties::state_out::Property<LedValues>,
        buzzer: properties::event_out_last::Property<Duration>,
        ds18x20: properties::state_in::Property<Ds18x20State>,
    }
    impl Properties {
        pub fn new() -> Self {
            Self {
                ins_changed_waker: properties::waker::InsChangedWaker::new(),
                outs_changed_waker: properties::waker::OutsChangedWaker::new(),

                keys: properties::state_event_in::Property::<KeyValues, KeyChangesCount>::new(),
                leds: properties::state_out::Property::<LedValues>::new([false; LED_COUNT]),
                buzzer: properties::event_out_last::Property::<Duration>::new(),
                ds18x20: properties::state_in::Property::<Ds18x20State>::new(),
            }
        }

        pub fn device_reset(&self) -> bool {
            self.leds.device_reset();

            false // break
                || self.keys.device_reset()
                || self.ds18x20.device_reset()
        }

        pub fn remote(&self) -> PropertiesRemote {
            PropertiesRemote {
                ins_changed_waker_remote: self.ins_changed_waker.remote(),
                outs_changed_waker_remote: self.outs_changed_waker.remote(),

                keys: self.keys.user_remote(),
                leds: self.leds.user_remote(),
                buzzer: self.buzzer.user_remote(),
                ds18x20: self.ds18x20.user_remote(),
            }
        }
    }

    #[derive(Debug)]
    pub struct Device {
        properties: Properties,

        poll_waker: waker_stream::mpsc::Signal,
    }
    impl Device {
        pub fn new() -> Self {
            Self {
                properties: Properties::new(),

                poll_waker: waker_stream::mpsc::Signal::new(),
            }
        }

        pub fn properties_remote(&self) -> PropertiesRemote {
            self.properties.remote()
        }

        async fn run(
            &self,
            exit_flag: async_flag::Receiver,
        ) -> Exited {
            // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
            let properties_outs_changed_waker_runner = self
                .properties
                .outs_changed_waker
                .stream()
                .stream_take_until_exhausted(exit_flag.clone())
                .for_each(async move |()| {
                    self.poll_waker.wake();
                })
                .boxed();

            let _: ((),) = join!(properties_outs_changed_waker_runner);

            Exited
        }
    }

    impl runner::Device for Device {
        fn device_type_name() -> &'static str {
            "JunctionBox_Minimal_v1"
        }
        fn address_device_type() -> AddressDeviceType {
            AddressDeviceType::new_from_ordinal(3).unwrap()
        }

        fn poll_waker(&self) -> Option<&waker_stream::mpsc::Signal> {
            Some(&self.poll_waker)
        }

        fn as_runnable(&self) -> Option<&dyn Runnable> {
            Some(self)
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
            let mut stage_2_properties_ins_changed = false;

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
                if self.properties.keys.device_set(
                    stage_2_response_keys.values(),
                    stage_2_response_keys.changes_count(),
                ) {
                    stage_2_properties_ins_changed = true;
                }
            }
            if let Some(stage_2_response_ds18x20) = stage_2_response_ds18x20 {
                if self
                    .properties
                    .ds18x20
                    .device_set(stage_2_response_ds18x20.state)
                {
                    stage_2_properties_ins_changed = true;
                }
            }

            if stage_2_properties_ins_changed {
                self.properties.ins_changed_waker.wake();
            }

            Ok(())
        }

        async fn deinitialize(
            &self,
            _driver: &ApplicationDriver<'_>,
        ) -> Result<(), Error> {
            Ok(())
        }

        fn reset(&self) {
            if self.properties.device_reset() {
                self.properties.ins_changed_waker.wake();
            }
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
                .collect::<ArrayVec<_, 8>>()
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
            if let Some(leds) = &self.leds {
                serializer.push_byte(b'L');
                leds.serialize(serializer);
            }
            if let Some(buzzer) = &self.buzzer {
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
        pub state: Ds18x20State,
    }
    impl BusResponseDs18x20 {
        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let state = Ds18x20SensorState::parse(parser)
                .context("state")?
                .into_inner();
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
            super::super::{
                super::houseblocks_v1::common::Payload,
                datatypes::ds18x20::{SensorType as Ds18x20SensorType, State as Ds18x20State},
            },
            BusResponse, BusResponseDs18x20, BusResponseKey, BusResponseKeys, BusResponsePoll,
        };
        use crate::datatypes::temperature::{Temperature, Unit as TemperatureUnit};

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
                    state: Ds18x20State {
                        sensor_type: Ds18x20SensorType::B,
                        reset_count: 0,
                        temperature: Some(
                            Temperature::from_unit(TemperatureUnit::Celsius, 125.00).unwrap(),
                        ),
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
