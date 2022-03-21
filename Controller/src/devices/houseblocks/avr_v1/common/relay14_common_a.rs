pub mod logic {
    use super::{super::super::logic::runner, hardware};
    use crate::{
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

        fn new(hardware_device: &hardware::Device<S::HardwareSpecification>) -> Device<S> {
            Device::new(hardware_device)
        }
    }

    #[derive(Debug)]
    pub struct Device<'h, S: Specification> {
        properties_remote: hardware::PropertiesRemote<'h>,

        signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
        signal_outputs: [signal::state_target_last::Signal<bool>; hardware::OUTPUT_COUNT],

        gui_summary_waker: waker_stream::mpmc::Sender,

        _phantom: PhantomData<S>,
    }
    impl<'h, S: Specification> Device<'h, S> {
        pub fn new(hardware_device: &'h hardware::Device<S::HardwareSpecification>) -> Self {
            Self {
                properties_remote: hardware_device.properties_remote(),

                signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
                signal_outputs: array_init(|_| signal::state_target_last::Signal::<bool>::new()),

                gui_summary_waker: waker_stream::mpmc::Sender::new(),

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
                .collect::<ArrayVec<_, { hardware::OUTPUT_COUNT }>>()
                .into_inner()
                .unwrap();
            if outputs_last.iter().any(|output_last| output_last.pending) {
                let outputs = outputs_last
                    .iter()
                    .map(|output_last| output_last.value.unwrap_or(false))
                    .collect::<ArrayVec<_, { hardware::OUTPUT_COUNT }>>()
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
                self.gui_summary_waker.wake()
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

    impl<'h, S: Specification> runner::Device for Device<'h, S> {
        type HardwareDevice = hardware::Device<S::HardwareSpecification>;

        fn class() -> &'static str {
            S::class()
        }

        fn as_runnable(&self) -> &dyn Runnable {
            self
        }
        fn as_gui_summary_provider(&self) -> Option<&dyn devices::GuiSummaryProvider> {
            Some(self)
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub enum SignalIdentifier {
        Output(usize),
    }
    impl signals::Identifier for SignalIdentifier {}
    impl<'h, S: Specification> signals::Device for Device<'h, S> {
        fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
            Some(&self.signals_targets_changed_waker)
        }
        fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
            None
        }

        type Identifier = SignalIdentifier;
        fn by_identifier(&self) -> signals::ByIdentifier<Self::Identifier> {
            self.signal_outputs
                .iter()
                .enumerate()
                .map(|(output_index, output_signal)| {
                    (
                        SignalIdentifier::Output(output_index),
                        output_signal as &dyn signal::Base,
                    )
                })
                .collect()
        }
    }

    #[async_trait]
    impl<'h, S: Specification> Runnable for Device<'h, S> {
        async fn run(
            &self,
            exit_flag: async_flag::Receiver,
        ) -> Exited {
            self.run(exit_flag).await
        }
    }

    #[derive(Serialize)]
    struct GuiSummary {
        values: [bool; hardware::OUTPUT_COUNT],
    }
    impl<'h, S: Specification> devices::GuiSummaryProvider for Device<'h, S> {
        fn value(&self) -> Box<dyn devices::GuiSummary> {
            let gui_summary = GuiSummary {
                values: self.properties_remote.outputs.peek_last(),
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
    use super::super::super::{
        super::houseblocks_v1::common::{AddressDeviceType, Payload},
        hardware::{driver::ApplicationDriver, parser::Parser, runner, serializer::Serializer},
        properties,
    };
    use crate::util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runtime::{Exited, Runnable},
        waker_stream,
    };
    use anyhow::{ensure, Context, Error};
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use futures::{future::FutureExt, join, stream::StreamExt};
    use std::{fmt, iter, marker::PhantomData, time::Duration};

    pub const OUTPUT_COUNT: usize = 14;
    pub type OutputValues = [bool; OUTPUT_COUNT];

    pub trait Specification: Send + Sync + fmt::Debug {
        fn device_type_name() -> &'static str;
        fn address_device_type() -> AddressDeviceType;
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
                    [false; OUTPUT_COUNT],
                ),
            }
        }

        pub fn device_reset(&self) {
            self.outputs.device_reset();
        }

        pub fn remote(&self) -> PropertiesRemote {
            PropertiesRemote {
                outs_changed_waker_remote: self.outs_changed_waker.remote(),

                outputs: self.outputs.user_remote(),
            }
        }
    }

    #[derive(Debug)]
    pub struct Device<S: Specification> {
        properties: Properties,

        poll_waker: waker_stream::mpsc_local::Signal,

        _phantom: PhantomData<S>,
    }
    impl<S: Specification> Device<S> {
        pub fn new() -> Self {
            Self {
                properties: Properties::new(),

                poll_waker: waker_stream::mpsc_local::Signal::new(),

                _phantom: PhantomData,
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
            let outs_changed_waker_runner = self
                .properties
                .outs_changed_waker
                .stream(false)
                .stream_take_until_exhausted(exit_flag)
                .for_each(async move |()| {
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

        fn poll_waker(&self) -> Option<&waker_stream::mpsc_local::Signal> {
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
                    .map(move |outputs| BusRequestOutputs { values: **outputs }),
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
        pub values: [bool; OUTPUT_COUNT],
    }
    impl BusRequestOutputs {
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
        pub outputs: Option<BusRequestOutputs>,
    }
    impl BusRequest {
        pub fn is_nop(&self) -> bool {
            self.outputs.is_none()
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
            if let Some(outputs) = self.outputs.as_ref() {
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
