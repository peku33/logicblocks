pub mod logic {
    use super::{super::logic::runner, hardware};
    use crate::{
        datatypes::resistance::Resistance,
        devices,
        signals::{self, signal},
        util::{
            async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
            async_flag,
            runnable::{Exited, Runnable},
        },
    };
    use array_init::array_init;
    use async_trait::async_trait;
    use futures::stream::StreamExt;
    use itertools::Itertools;
    use serde::Serialize;
    use serde_big_array::BigArray;

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

        signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
        signal_inputs: [signal::state_source::Signal<Resistance>; hardware::INPUTS_COUNT],

        gui_summary_waker: devices::gui_summary::Waker,
    }

    impl<'h> Device<'h> {
        pub fn new(hardware_device: &'h hardware::Device) -> Self {
            Self {
                properties_remote: hardware_device.properties_remote(),

                signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
                signal_inputs: array_init(|_input_index| {
                    signal::state_source::Signal::<Resistance>::new(None)
                }),

                gui_summary_waker: devices::gui_summary::Waker::new(),
            }
        }

        fn properties_ins_changed(&self) {
            let mut signals_sources_changed = false;
            let mut gui_summary_changed = false;

            if let Some(inputs) = self.properties_remote.inputs.take_pending() {
                if let Some(inputs) = inputs {
                    self.signal_inputs
                        .iter()
                        .zip_eq(inputs)
                        .for_each(|(signal_input, input)| {
                            if signal_input.set_one(Some(input)) {
                                signals_sources_changed = true;
                            }
                        })
                } else {
                    self.signal_inputs.iter().for_each(|signal_input| {
                        if signal_input.set_one(None) {
                            signals_sources_changed = true;
                        }
                    });
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
            self.properties_remote
                .ins_changed_waker_remote
                .stream()
                .stream_take_until_exhausted(exit_flag.clone())
                .for_each(async |()| {
                    self.properties_ins_changed();
                })
                .await;

            Exited
        }
    }

    impl<'h> runner::Device for Device<'h> {
        type HardwareDevice = hardware::Device;

        fn class() -> &'static str {
            "reed_switch_v1"
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
        Input(usize),
    }
    impl signals::Identifier for SignalIdentifier {}
    impl<'h> signals::Device for Device<'h> {
        fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
            None
        }
        fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
            Some(&self.signals_sources_changed_waker)
        }

        type Identifier = SignalIdentifier;
        fn by_identifier(&self) -> signals::ByIdentifier<Self::Identifier> {
            self.signal_inputs
                .iter()
                .enumerate()
                .map(|(input_index, signal_input)| {
                    (
                        SignalIdentifier::Input(input_index),
                        signal_input as &dyn signal::Base,
                    )
                })
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
    #[serde(transparent)]
    pub struct GuiSummaryInputs {
        #[serde(with = "BigArray")]
        inner: [Resistance; hardware::INPUTS_COUNT],
    }
    #[derive(Debug, Serialize)]
    pub struct GuiSummary {
        inputs: Option<GuiSummaryInputs>,
    }
    impl<'h> devices::gui_summary::Device for Device<'h> {
        fn waker(&self) -> &devices::gui_summary::Waker {
            &self.gui_summary_waker
        }

        type Value = GuiSummary;
        fn value(&self) -> Self::Value {
            let inputs = self
                .properties_remote
                .inputs
                .peek_last()
                .map(|inputs| GuiSummaryInputs { inner: inputs });

            Self::Value { inputs }
        }
    }
}

pub mod hardware {
    use super::super::{
        super::houseblocks_v1::common::{AddressDeviceType, Payload},
        hardware::{driver::ApplicationDriver, parser::Parser, runner, serializer::Serializer},
        properties,
    };
    use crate::{
        datatypes::resistance::Resistance,
        util::{
            async_flag, async_waker,
            runnable::{Exited, Runnable},
        },
    };
    use anyhow::{bail, Context, Error};
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use std::time::Duration;

    pub const INPUTS_COUNT: usize = 40;

    pub type InputValue = Resistance;
    pub type InputValues = [InputValue; INPUTS_COUNT];

    #[derive(Debug)]
    pub struct PropertiesRemote<'p> {
        pub ins_changed_waker_remote: properties::waker::InsChangedWakerRemote<'p>,

        pub inputs: properties::state_in::Remote<'p, InputValues>,
    }

    #[derive(Debug)]
    pub struct Properties {
        ins_changed_waker: properties::waker::InsChangedWaker,

        inputs: properties::state_in::Property<InputValues>,
    }
    impl Properties {
        pub fn new() -> Self {
            Self {
                ins_changed_waker: properties::waker::InsChangedWaker::new(),

                inputs: properties::state_in::Property::<InputValues>::new(),
            }
        }

        pub fn device_reset(&self) -> bool {
            self.inputs.device_reset()
        }

        pub fn remote(&self) -> PropertiesRemote {
            PropertiesRemote {
                ins_changed_waker_remote: self.ins_changed_waker.remote(),

                inputs: self.inputs.user_remote(),
            }
        }
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

        pub fn properties_remote(&self) -> PropertiesRemote {
            self.properties.remote()
        }
    }

    impl runner::Device for Device {
        fn device_type_name() -> &'static str {
            "ReedSwitch_v1"
        }
        fn address_device_type() -> AddressDeviceType {
            AddressDeviceType::new_from_ordinal(2).unwrap()
        }

        fn poll_waker(&self) -> Option<&async_waker::mpsc::Signal> {
            None
        }

        fn as_runnable(&self) -> Option<&dyn Runnable> {
            None
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
            Some(Duration::from_secs(1))
        }
        async fn poll(
            &self,
            driver: &ApplicationDriver<'_>,
        ) -> Result<(), Error> {
            // request phase
            // always used, to check device uptime
            let poll_request = BusRequest::Poll;
            let poll_request_payload = poll_request.to_payload();
            let poll_response_payload = driver
                .transaction_out_in(poll_request_payload, None)
                .await
                .context("poll transaction")?;
            let poll_response = BusResponse::from_payload(&poll_response_payload, &poll_request)
                .context("poll response")?;
            let poll_response_poll = match poll_response {
                BusResponse::Poll(bus_response_poll) => bus_response_poll,
                _ => bail!("invalid poll response type"),
            };

            // stage 2
            // used when changed or no values available
            if !(poll_response_poll.changed || self.properties.inputs.device_must_read()) {
                return Ok(());
            }

            let read_request = BusRequest::Read;
            let read_request_payload = read_request.to_payload();
            let read_response_payload = driver
                .transaction_out_in(read_request_payload, None)
                .await
                .context("read transaction")?;
            let read_response = BusResponse::from_payload(&read_response_payload, &read_request)
                .context("read response")?;
            let read_response_read = match read_response {
                BusResponse::Read(bus_response_read) => bus_response_read,
                _ => bail!("invalid read response type"),
            };

            if self.properties.inputs.device_set(read_response_read.inputs) {
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

    #[derive(PartialEq, Eq, Debug)]
    enum BusRequest {
        Poll,
        Read,
    }
    impl BusRequest {
        pub fn to_payload(&self) -> Payload {
            let mut serializer = Serializer::new();
            self.serialize(&mut serializer);
            serializer.into_payload()
        }

        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            match self {
                BusRequest::Poll => {
                    serializer.push_byte(b'P');
                }
                BusRequest::Read => {
                    serializer.push_byte(b'R');
                }
            }
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponsePoll {
        changed: bool,
    }
    impl BusResponsePoll {
        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let changed = parser.expect_bool().context("changed")?;
            Ok(Self { changed })
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponseRead {
        inputs: InputValues,
    }
    impl BusResponseRead {
        pub fn transform_input_raw(input_raw: u8) -> Result<InputValue, Error> {
            const R_1: f64 = 2200.0;
            const R_MUX: f64 = 100.0;
            const V_IN: f64 = 5.0;

            let v_out = (input_raw as f64) / (u8::MAX as f64) * V_IN;

            let r_mux_r_2 = (v_out * R_1) / (V_IN - v_out);
            let r_2 = (r_mux_r_2 - R_MUX).max(0.0);

            let input = InputValue::from_ohms(r_2).unwrap();

            Ok(input)
        }
        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let inputs = (0..INPUTS_COUNT)
                .map(|_input_index| -> Result<_, Error> {
                    let input_raw = parser.expect_u8().context("expect_u8")?;
                    let input =
                        Self::transform_input_raw(input_raw).context("transform_input_raw")?;
                    Ok(input)
                })
                .collect::<Result<ArrayVec<_, { INPUTS_COUNT }>, _>>()
                .context("collect")?
                .into_inner()
                .unwrap();
            Ok(Self { inputs })
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    enum BusResponse {
        Poll(BusResponsePoll),
        Read(BusResponseRead),
    }
    impl BusResponse {
        pub fn from_payload(
            payload: &Payload,
            bus_request: &BusRequest,
        ) -> Result<Self, Error> {
            let mut parser = Parser::from_payload(payload);
            let self_ = Self::parse(&mut parser, bus_request).context("parse")?;
            Ok(self_)
        }

        pub fn parse(
            parser: &mut Parser,
            bus_request: &BusRequest,
        ) -> Result<Self, Error> {
            let self_ = match bus_request {
                BusRequest::Poll => {
                    let bus_response_poll = BusResponsePoll::parse(parser).context("parse")?;
                    let self_ = Self::Poll(bus_response_poll);
                    self_
                }
                BusRequest::Read => {
                    let bus_response_read = BusResponseRead::parse(parser).context("parse")?;
                    let self_ = Self::Read(bus_response_read);
                    self_
                }
            };

            parser.expect_end().context("expect_end")?;

            Ok(self_)
        }
    }
    #[cfg(test)]
    mod tests_bus_response {
        use super::{
            super::super::super::houseblocks_v1::common::Payload, BusRequest, BusResponse,
            BusResponsePoll, InputValue,
        };
        use approx::assert_relative_eq;

        #[test]
        fn poll_1() {
            let bus_request = BusRequest::Poll;

            let payload = Payload::new(Box::from(*b"1")).unwrap();
            let bus_response = BusResponse::from_payload(&payload, &bus_request).unwrap();

            let bus_response_expected = BusResponse::Poll(BusResponsePoll { changed: true });

            assert_eq!(bus_response, bus_response_expected);
        }
        #[test]
        fn read_1() {
            let bus_request = BusRequest::Read;

            let payload = Payload::new(Box::from(*b"FF000988030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F202122232420E0")).unwrap();
            let bus_response = BusResponse::from_payload(&payload, &bus_request).unwrap();

            let bus_response_read = match bus_response {
                BusResponse::Read(bus_response_read) => bus_response_read,
                _ => panic!("invalid bus response type"),
            };

            assert_eq!(bus_response_read.inputs[0], InputValue::infinity());
            assert_eq!(bus_response_read.inputs[1], InputValue::zero());
            assert_relative_eq!(bus_response_read.inputs[2].to_ohms(), 0.0, epsilon = 1e-2);
            assert_relative_eq!(
                bus_response_read.inputs[3].to_ohms(),
                2414.2857142857138,
                epsilon = 1e-2
            );
            assert_relative_eq!(
                bus_response_read.inputs[38].to_ohms(),
                215.69506726457394,
                epsilon = 1e-2
            );
            assert_relative_eq!(
                bus_response_read.inputs[39].to_ohms(),
                15796.7741935484,
                epsilon = 1e-2
            );
        }
    }
}
