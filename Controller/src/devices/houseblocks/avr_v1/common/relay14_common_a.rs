pub mod logic {
    use super::{super::super::logic, hardware};
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

    pub trait Specification: Send + Sync + fmt::Debug {
        type HardwareSpecification: hardware::Specification;

        fn class() -> &'static str;
    }

    #[derive(Debug)]
    pub struct Device<S: Specification> {
        properties_remote: hardware::PropertiesRemote,
        properties_remote_out_changed_waker: waker_stream::mpsc::SenderReceiver,

        signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
        signal_outputs: [signal::state_target_last::Signal<bool>; hardware::OUTPUT_COUNT],

        gui_summary_waker: waker_stream::mpmc::Sender,

        _phantom: PhantomData<S>,
    }
    impl<S: Specification> Device<S> {
        fn signals_targets_changed(&self) {
            let mut properties_remote_changed = false;
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
                    properties_remote_changed = true;
                    gui_summary_changed = true;
                }
            }

            if properties_remote_changed {
                self.properties_remote_out_changed_waker.wake();
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

    impl<S: Specification> logic::Device for Device<S> {
        type HardwareDevice = hardware::Device<S::HardwareSpecification>;

        fn new(properties_remote: hardware::PropertiesRemote) -> Self {
            Self {
                properties_remote,
                properties_remote_out_changed_waker: waker_stream::mpsc::SenderReceiver::new(),

                signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
                signal_outputs: array_init(|_| signal::state_target_last::Signal::<bool>::new()),

                gui_summary_waker: waker_stream::mpmc::Sender::new(),

                _phantom: PhantomData,
            }
        }

        fn class() -> &'static str {
            S::class()
        }

        fn as_runnable(&self) -> &dyn Runnable {
            self
        }
        fn as_gui_summary_provider(&self) -> Option<&dyn devices::GuiSummaryProvider> {
            Some(self)
        }
        fn properties_remote_in_changed(&self) {
            // We have no "in" properties
        }
        fn properties_remote_out_changed_waker_receiver(
            &self
        ) -> waker_stream::mpsc::ReceiverLease {
            self.properties_remote_out_changed_waker.receiver()
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub enum SignalIdentifier {
        Output(usize),
    }
    impl signals::Identifier for SignalIdentifier {}
    impl<S: Specification> signals::Device for Device<S> {
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
    impl<S: Specification> Runnable for Device<S> {
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
    impl<S: Specification> devices::GuiSummaryProvider for Device<S> {
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
        hardware::{
            driver::ApplicationDriver, parser::Parser, property, runner, serializer::Serializer,
        },
    };
    use anyhow::{ensure, Context, Error};
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use std::{fmt, iter, marker::PhantomData, time::Duration};

    pub const OUTPUT_COUNT: usize = 14;
    pub type OutputValues = [bool; OUTPUT_COUNT];

    pub trait Specification: Send + Sync + fmt::Debug {
        fn device_type_name() -> &'static str;
        fn address_device_type() -> AddressDeviceType;
    }

    #[derive(Debug)]
    pub struct Properties {
        outputs: property::state_out::Property<OutputValues>,
    }
    impl Properties {
        pub fn new() -> Self {
            Self {
                outputs: property::state_out::Property::<OutputValues>::new([false; OUTPUT_COUNT]),
            }
        }
    }
    impl runner::Properties for Properties {
        fn user_pending(&self) -> bool {
            false
        }
        fn device_reset(&self) {
            self.outputs.device_reset();
        }

        type Remote = PropertiesRemote;
        fn remote(&self) -> Self::Remote {
            PropertiesRemote {
                outputs: self.outputs.user_sink(),
            }
        }
    }
    #[derive(Debug)]
    pub struct PropertiesRemote {
        pub outputs: property::state_out::Sink<OutputValues>,
    }

    #[derive(Debug)]
    pub struct Device<S: Specification> {
        properties: Properties,
        _phantom: PhantomData<S>,
    }
    impl<S: Specification> Device<S> {
        pub fn new() -> Self {
            Self {
                properties: Properties::new(),
                _phantom: PhantomData,
            }
        }
    }
    impl<S: Specification> runner::Device for Device<S> {
        fn device_type_name() -> &'static str {
            S::device_type_name()
        }
        fn address_device_type() -> AddressDeviceType {
            S::address_device_type()
        }

        type Properties = Properties;
        fn properties(&self) -> &Self::Properties {
            &self.properties
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

        fn failed(&self) {}
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
