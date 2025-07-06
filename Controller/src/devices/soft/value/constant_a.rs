use crate::{
    devices,
    signals::{self, signal, types::state::Value},
    util::{
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use maplit::hashmap;
use std::{any::type_name, borrow::Cow};

#[derive(Debug)]
pub struct Configuration<V>
where
    V: Value + Clone,
{
    pub value: V,
}

#[derive(Debug)]
pub struct Device<V>
where
    V: Value + Clone,
{
    configuration: Configuration<V>,

    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_output: signal::state_source::Signal<V>,
}
impl<V> Device<V>
where
    V: Value + Clone,
{
    pub fn new(configuration: Configuration<V>) -> Self {
        let value = configuration.value.clone();

        Self {
            configuration,

            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_output: signal::state_source::Signal::<V>::new(Some(value)),
        }
    }
}

impl<V> devices::Device for Device<V>
where
    V: Value + Clone,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!("soft/value/constant_a<{}>", type_name::<V>()))
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase {
        self
    }
}

#[async_trait]
impl<V> Runnable for Device<V>
where
    V: Value + Clone,
{
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        exit_flag.await;
        Exited
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SignalIdentifier {
    Output,
}
impl signals::Identifier for SignalIdentifier {}
impl<V> signals::Device for Device<V>
where
    V: Value + Clone,
{
    fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
        None
    }
    fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
        Some(&self.signals_sources_changed_waker)
    }

    type Identifier = SignalIdentifier;
    fn by_identifier(&self) -> signals::ByIdentifier<'_, Self::Identifier> {
        hashmap! {
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}
