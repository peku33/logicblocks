pub mod logic {
    use super::{super::super::logic, hardware};
    use crate::{
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

        signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
        signal_outputs: [signal::state_target_last::Signal<bool>; hardware::OUTPUT_COUNT],

        gui_summary_waker: waker_stream::mpmc::Sender,

        _phantom: PhantomData<S>,
    }
    impl<S: Specification> logic::Device for Device<S> {
        type HardwareDevice = hardware::Device<S::HardwareSpecification>;

        fn new(properties_remote: hardware::PropertiesRemote) -> Self {
            Self {
                properties_remote,
                properties_remote_out_changed_waker: waker_stream::mpsc::SenderReceiver::new(),

                signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
                signal_outputs: array_init(|_| signal::state_target_last::Signal::<bool>::new()),

                gui_summary_waker: waker_stream::mpmc::Sender::new(),

                _phantom: PhantomData,
            }
        }

        fn class() -> &'static str {
            S::class()
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
    #[async_trait]
    impl<S: Specification> Runnable for Device<S> {
        async fn run(
            &self,
            exit_flag: async_flag::Receiver,
        ) -> Exited {
            exit_flag.await;
            Exited
        }
    }
    impl<S: Specification> signals::Device for Device<S> {
        fn signal_targets_changed_wake(&self) {
            let mut properties_remote_changed = false;
            let mut gui_summary_changed = false;

            // outputs
            let outputs_last = self
                .signal_outputs
                .iter()
                .map(|signal_output| signal_output.take_last())
                .collect::<ArrayVec<_, { hardware::OUTPUT_COUNT }>>();
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
        fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
            self.signal_sources_changed_waker.receiver()
        }
        fn signals(&self) -> signals::Signals {
            self.signal_outputs
                .iter()
                .enumerate()
                .map(|(signal_id, signal)| {
                    (
                        signal_id as signals::Id,
                        signal as &dyn signals::signal::Base,
                    )
                })
                .collect::<signals::Signals>()
        }
    }

    #[derive(Serialize)]
    struct GuiSummary {
        values: [bool; hardware::OUTPUT_COUNT],
    }
    impl<S: Specification> devices::GuiSummaryProvider for Device<S> {
        fn value(&self) -> Box<dyn devices::GuiSummary> {
            let value = GuiSummary {
                values: self.properties_remote.outputs.get_last(),
            };
            let value = Box::new(value);
            value
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
    use anyhow::{Context, Error};
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use maplit::hashmap;
    use std::{collections::HashMap, fmt, marker::PhantomData, time::Duration};

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
                outputs: property::state_out::Property::new([false; OUTPUT_COUNT]),
            }
        }
    }
    impl runner::Properties for Properties {
        fn by_name(&self) -> HashMap<&'static str, &dyn property::Base> {
            hashmap! {
                "outputs" => &self.outputs as &dyn property::Base,
            }
        }

        fn in_any_user_pending(&self) -> bool {
            false
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
    impl<S: Specification> runner::Device for Device<S> {
        fn new() -> Self {
            Self {
                properties: Properties::new(),
                _phantom: PhantomData,
            }
        }
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
    impl<S: Specification> Runnable for Device<S> {
        async fn run(
            &self,
            exit_flag: async_flag::Receiver,
        ) -> Exited {
            exit_flag.await;
            Exited
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
            // Stage 1
            let outputs_pending = self.properties.outputs.device_pending();

            let request = BusRequest::new(
                outputs_pending
                    .as_ref()
                    .map(move |outputs| BusRequestOutputs::new(**outputs)),
            );
            let request_payload = request.into_payload();
            let response_payload = driver
                .transaction_out_in(request_payload, None)
                .await
                .context("transaction")?;
            let _response = BusResponse::from_payload(response_payload).context("response")?;

            // Propagate values to properties
            if let Some(outputs_pending) = outputs_pending {
                outputs_pending.commit();
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
            self.properties.outputs.device_reset();
        }
    }

    struct BusRequestOutputs {
        values: [bool; OUTPUT_COUNT],
    }
    impl BusRequestOutputs {
        pub fn new(values: [bool; OUTPUT_COUNT]) -> Self {
            Self { values }
        }
        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            let mut values = ArrayVec::<bool, 16>::new();
            values.try_extend_from_slice(&self.values).unwrap();
            values.push(false);
            values.push(false);
            serializer.push_bool_array_16(values);
        }
    }

    struct BusRequest {
        outputs: Option<BusRequestOutputs>,
    }
    impl BusRequest {
        pub fn new(outputs: Option<BusRequestOutputs>) -> Self {
            Self { outputs }
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
            if let Some(outputs) = self.outputs.as_ref() {
                serializer.push_byte(b'H');
                outputs.serialize(serializer);
            }
        }
    }

    struct BusResponse {}
    impl BusResponse {
        pub fn from_payload(payload: Payload) -> Result<Self, Error> {
            let mut parser = ParserPayload::new(&payload);
            let self_ = Self::parse(&mut parser)?;
            Ok(self_)
        }
        pub fn parse(parser: &mut impl Parser) -> Result<Self, Error> {
            parser.expect_end()?;
            Ok(Self {})
        }
    }
}
