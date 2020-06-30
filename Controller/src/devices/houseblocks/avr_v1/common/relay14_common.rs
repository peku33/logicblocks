pub mod logic {
    use super::{super::super::logic, hardware};
    use crate::{
        datatypes::boolean::Boolean,
        logic::{
            device::{SignalId, Signals},
            signal,
            signal::SignalBase,
        },
        web::{
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
    use std::marker::PhantomData;

    pub trait Specification: Send + Sync {
        type HardwareSpecification: hardware::Specification;

        fn class() -> &'static str;
    }

    pub struct Device<S: Specification> {
        outputs: [signal::state_target::Signal<Boolean>; hardware::OUTPUT_COUNT],
        _phantom: PhantomData<S>,
    }
    #[async_trait]
    impl<S: Specification> logic::Device for Device<S> {
        type HardwareDevice = hardware::Device<S::HardwareSpecification>;

        fn new() -> Self {
            let outputs = array_init(|_| signal::state_target::Signal::new());

            Self {
                outputs,
                _phantom: PhantomData,
            }
        }
        fn class() -> &'static str {
            S::class()
        }

        fn signals(&self) -> Signals {
            self.outputs
                .iter()
                .enumerate()
                .map(|(index, signal)| (index as SignalId, signal as &dyn SignalBase))
                .collect::<Signals>()
        }

        async fn run(
            &self,
            remote_properties: hardware::RemoteProperties<'_>,
        ) -> ! {
            let hardware::RemoteProperties { outputs } = remote_properties;

            let outputs_ref = &outputs;
            let outputs_runner = self
                .outputs
                .iter()
                .map(|output| output.stream().map(|_| ()))
                .collect::<SelectAll<_>>()
                .map(|()| {
                    array_init(|index| {
                        self.outputs[index]
                            .current()
                            .map(|value| value.into())
                            .unwrap_or(false)
                    })
                })
                .for_each(async move |value| outputs_ref.set(value));
            pin_mut!(outputs_runner);

            select! {
                () = outputs_runner => panic!("outputs_runner yielded"),
            }
        }
        async fn finalize(self) {}
    }
    impl<S: Specification> Handler for Device<S> {
        fn handle(
            &self,
            _request: Request,
            _uri_cursor: UriCursor,
        ) -> BoxFuture<'static, Response> {
            async move { Response::ok_empty() }.boxed()
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
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use failure::Error;
    use futures::{pin_mut, select, stream::StreamExt};
    use std::{marker::PhantomData, time::Duration};

    pub const OUTPUT_COUNT: usize = 14;
    pub type OutputValues = [bool; OUTPUT_COUNT];

    pub trait Specification: Send + Sync {
        fn device_type_name() -> &'static str;
        fn address_device_type() -> AddressDeviceType;
    }

    pub struct Device<S: Specification> {
        outputs: property::state_out::Property<OutputValues>,
        _phantom: PhantomData<S>,
    }
    pub struct RemoteProperties<'d> {
        pub outputs: property::state_out::ValueSink<'d, OutputValues>,
    }
    #[async_trait]
    impl<S: Specification> runner::Device for Device<S> {
        fn new() -> Self {
            Self {
                outputs: property::state_out::Property::new([false; OUTPUT_COUNT]),
                _phantom: PhantomData,
            }
        }

        fn device_type_name() -> &'static str {
            S::device_type_name()
        }

        fn address_device_type() -> AddressDeviceType {
            S::address_device_type()
        }

        type RemoteProperties<'d> = RemoteProperties<'d>;
        fn remote_properties(&self) -> Self::RemoteProperties<'_> {
            RemoteProperties {
                outputs: self.outputs.user_get_sink(),
            }
        }

        async fn run(
            &self,
            run_context: &dyn runner::RunContext,
        ) -> ! {
            let outputs_runner = self.outputs.device_get_stream().for_each(async move |()| {
                run_context.poll_request();
            });
            pin_mut!(outputs_runner);

            select! {
                () = outputs_runner => panic!("outputs_runner yielded"),
            }
        }
        async fn finalize(self) {}
    }
    #[async_trait]
    impl<S: Specification> runner::BusDevice for Device<S> {
        async fn initialize(
            &self,
            _driver: &dyn ApplicationDriver,
        ) -> Result<(), Error> {
            Ok(())
        }

        fn poll_delay(&self) -> Option<Duration> {
            None
        }
        async fn poll(
            &self,
            driver: &dyn ApplicationDriver,
        ) -> Result<(), Error> {
            let outputs_pending = self.outputs.device_get_pending();

            let request = BusRequest::new(
                outputs_pending
                    .as_ref()
                    .map(move |outputs| BusRequestOutputs::new(**outputs)),
            );
            let request_payload = request.into_payload();
            let response_payload = driver.transaction_out_in(request_payload, None).await?;
            let _response = BusResponse::from_payload(response_payload)?;

            if let Some(outputs_pending) = outputs_pending {
                outputs_pending.commit();
            }

            Ok(())
        }

        async fn deinitialize(
            &self,
            _driver: &dyn ApplicationDriver,
        ) -> Result<(), Error> {
            Ok(())
        }

        fn failed(&self) {}
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
            let mut values = ArrayVec::<[bool; 16]>::new();
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
