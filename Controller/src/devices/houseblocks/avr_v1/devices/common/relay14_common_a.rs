pub mod logic {
    use super::{super::super::super::logic::runner, hardware};
    use crate::{
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
    use futures::stream::StreamExt;
    use serde::Serialize;
    use std::{fmt, marker::PhantomData};

    pub trait Specification: Send + Sync + fmt::Debug + 'static {
        type HardwareSpecification: hardware::Specification;

        fn class() -> &'static str;
    }

    #[derive(Debug)]
    pub struct DeviceFactory<S: Specification> {
        _phantom: PhantomData<S>,
    }
    impl<S: Specification> runner::DeviceFactory for DeviceFactory<S> {
        type Device<'h> = Device<'h, S>;

        fn new(hardware_device: &hardware::Device<S::HardwareSpecification>) -> Device<'_, S> {
            Device::new(hardware_device)
        }
    }

    #[derive(Debug)]
    pub struct Device<'h, S: Specification> {
        properties_remote: hardware::PropertiesRemote<'h>,

        signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
        signal_outputs: [signal::state_target_last::Signal<bool>; hardware::OUTPUTS_COUNT],

        gui_summary_waker: devices::gui_summary::Waker,

        _phantom: PhantomData<S>,
    }
    impl<'h, S: Specification> Device<'h, S> {
        pub fn new(hardware_device: &'h hardware::Device<S::HardwareSpecification>) -> Self {
            Self {
                properties_remote: hardware_device.properties_remote(),

                signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
                signal_outputs: array_init(|_output_index| {
                    signal::state_target_last::Signal::<bool>::new()
                }),

                gui_summary_waker: devices::gui_summary::Waker::new(),

                _phantom: PhantomData,
            }
        }

        fn signals_targets_changed(&self) {
            let mut properties_outs_changed = false;
            let mut gui_summary_changed = false;

            // outputs
            let outputs_last = self
                .signal_outputs
                .iter()
                .map(|signal_output| signal_output.take_last())
                .collect::<ArrayVec<_, { hardware::OUTPUTS_COUNT }>>()
                .into_inner()
                .unwrap();
            if outputs_last.iter().any(|output_last| output_last.pending) {
                let outputs = outputs_last
                    .iter()
                    .map(|output_last| output_last.value.unwrap_or(false))
                    .collect::<ArrayVec<_, { hardware::OUTPUTS_COUNT }>>()
                    .into_inner()
                    .unwrap();

                if self.properties_remote.outputs.set(outputs) {
                    properties_outs_changed = true;
                    gui_summary_changed = true;
                }
            }

            if properties_outs_changed {
                self.properties_remote.outs_changed_waker_remote.wake();
            }
            if gui_summary_changed {
                self.gui_summary_waker.wake();
            }
        }

        async fn run(
            &self,
            exit_flag: async_flag::Receiver,
        ) -> Exited {
            self.signals_targets_changed_waker
                .stream()
                .stream_take_until_exhausted(exit_flag)
                .for_each(async |()| {
                    self.signals_targets_changed();
                })
                .await;

            Exited
        }
    }

    impl<S: Specification> runner::Device for Device<'_, S> {
        type HardwareDevice = hardware::Device<S::HardwareSpecification>;

        fn class() -> &'static str {
            S::class()
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
        Output(usize),
    }
    impl signals::Identifier for SignalIdentifier {}
    impl<S: Specification> signals::Device for Device<'_, S> {
        fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
            Some(&self.signals_targets_changed_waker)
        }
        fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
            None
        }

        type Identifier = SignalIdentifier;
        fn by_identifier(&self) -> signals::ByIdentifier<'_, Self::Identifier> {
            self.signal_outputs
                .iter()
                .enumerate()
                .map(|(output_index, signal_output)| {
                    (
                        SignalIdentifier::Output(output_index),
                        signal_output as &dyn signal::Base,
                    )
                })
                .collect::<signals::ByIdentifier<_>>()
        }
    }

    #[async_trait]
    impl<S: Specification> Runnable for Device<'_, S> {
        async fn run(
            &self,
            exit_flag: async_flag::Receiver,
        ) -> Exited {
            self.run(exit_flag).await
        }
    }

    #[derive(Debug, Serialize)]
    pub struct GuiSummary {
        outputs: [bool; hardware::OUTPUTS_COUNT],
    }
    impl<S: Specification> devices::gui_summary::Device for Device<'_, S> {
        fn waker(&self) -> &devices::gui_summary::Waker {
            &self.gui_summary_waker
        }

        type Value = GuiSummary;
        fn value(&self) -> Self::Value {
            let outputs = self.properties_remote.outputs.peek_last();

            Self::Value { outputs }
        }
    }
}
pub mod hardware {
    use super::super::super::super::{
        super::houseblocks_v1::common::{AddressDeviceType, Payload},
        hardware::{
            driver::{ApplicationDriver, Firmware},
            parser::Parser,
            runner,
            serializer::Serializer,
        },
        properties,
    };
    use crate::util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag, async_waker,
        runnable::{Exited, Runnable},
    };
    use anyhow::{Context, Error, ensure};
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use futures::{future::FutureExt, join, stream::StreamExt};
    use std::{fmt, iter, marker::PhantomData, time::Duration};

    pub const OUTPUTS_COUNT: usize = 14;
    pub type OutputValues = [bool; OUTPUTS_COUNT];

    pub trait Specification: Send + Sync + fmt::Debug {
        fn device_type_name() -> &'static str;
        fn address_device_type() -> AddressDeviceType;
        fn firmware() -> Option<&'static Firmware<'static>>;
    }

    #[derive(Debug)]
    pub struct PropertiesRemote<'p> {
        pub outs_changed_waker_remote: properties::waker::OutsChangedWakerRemote<'p>,

        pub outputs: properties::state_out::Remote<'p, OutputValues>,
    }

    #[derive(Debug)]
    pub struct Properties {
        outs_changed_waker: properties::waker::OutsChangedWaker,

        outputs: properties::state_out::Property<OutputValues>,
    }
    impl Properties {
        pub fn new() -> Self {
            Self {
                outs_changed_waker: properties::waker::OutsChangedWaker::new(),

                outputs: properties::state_out::Property::<OutputValues>::new(
                    [false; OUTPUTS_COUNT],
                ),
            }
        }

        pub fn device_reset(&self) {
            self.outputs.device_reset();
        }

        pub fn remote(&self) -> PropertiesRemote<'_> {
            PropertiesRemote {
                outs_changed_waker_remote: self.outs_changed_waker.remote(),

                outputs: self.outputs.user_remote(),
            }
        }
    }

    #[derive(Debug)]
    pub struct Device<S: Specification> {
        properties: Properties,

        poll_waker: async_waker::mpsc::Signal,

        _phantom: PhantomData<S>,
    }
    impl<S: Specification> Device<S> {
        pub fn new() -> Self {
            Self {
                properties: Properties::new(),

                poll_waker: async_waker::mpsc::Signal::new(),

                _phantom: PhantomData,
            }
        }

        pub fn properties_remote(&self) -> PropertiesRemote<'_> {
            self.properties.remote()
        }

        async fn run(
            &self,
            exit_flag: async_flag::Receiver,
        ) -> Exited {
            // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
            let outs_changed_waker_runner = self
                .properties
                .outs_changed_waker
                .stream()
                .stream_take_until_exhausted(exit_flag)
                .for_each(async |()| {
                    self.poll_waker.wake();
                })
                .boxed();

            let _: ((),) = join!(outs_changed_waker_runner);

            Exited
        }
    }

    impl<S: Specification> runner::Device for Device<S> {
        fn device_type_name() -> &'static str {
            S::device_type_name()
        }
        fn address_device_type() -> AddressDeviceType {
            S::address_device_type()
        }
        fn firmware() -> Option<&'static Firmware<'static>> {
            S::firmware()
        }
        fn application_version_supported() -> Option<u16> {
            Some(2)
        }

        fn poll_waker(&self) -> Option<&async_waker::mpsc::Signal> {
            Some(&self.poll_waker)
        }

        fn as_runnable(&self) -> Option<&dyn Runnable> {
            Some(self)
        }
    }

    #[async_trait]
    impl<S: Specification> runner::BusDevice for Device<S> {
        async fn initialize(
            &self,
            _driver: &ApplicationDriver<'_>,
        ) -> Result<(), Error> {
            Ok(())
        }

        fn poll_delay(&self) -> Option<Duration> {
            None
        }
        async fn poll(
            &self,
            driver: &ApplicationDriver<'_>,
        ) -> Result<(), Error> {
            let outputs_pending = self.properties.outputs.device_pending();

            let request = BusRequest {
                outputs: outputs_pending
                    .as_ref()
                    .map(|outputs| BusRequestOutputs { values: **outputs }),
            };
            // we make a request no matter if it's required or not to confirm device is up
            let request_payload = request.to_payload();
            let response_payload = driver
                .transaction_out_in(request_payload, None)
                .await
                .context("transaction")?;
            let response = BusResponse::from_payload(&response_payload).context("response")?;

            // Propagate values to properties
            if let Some(outputs_pending) = outputs_pending {
                outputs_pending.commit()
            }

            ensure!(response == BusResponse {});

            Ok(())
        }

        async fn deinitialize(
            &self,
            _driver: &ApplicationDriver<'_>,
        ) -> Result<(), Error> {
            Ok(())
        }

        fn reset(&self) {
            self.properties.device_reset();
        }
    }

    #[async_trait]
    impl<S: Specification> Runnable for Device<S> {
        async fn run(
            &self,
            exit_flag: async_flag::Receiver,
        ) -> Exited {
            self.run(exit_flag).await
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusRequestOutputs {
        pub values: [bool; OUTPUTS_COUNT],
    }
    impl BusRequestOutputs {
        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            let values = self
                .values
                .iter()
                .copied()
                .chain(iter::repeat(false))
                .take(16)
                .collect::<ArrayVec<_, 16>>()
                .into_inner()
                .unwrap();
            serializer.push_bool_array_16(values);
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusRequest {
        pub outputs: Option<BusRequestOutputs>,
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
            if let Some(outputs) = &self.outputs {
                serializer.push_byte(b'H');
                outputs.serialize(serializer);
            }
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponse {}
    impl BusResponse {
        pub fn from_payload(payload: &Payload) -> Result<Self, Error> {
            let mut parser = Parser::from_payload(payload);
            let self_ = Self::parse(&mut parser).context("parse")?;
            Ok(self_)
        }
        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            parser.expect_end().context("expect_end")?;
            Ok(Self {})
        }
    }
}
