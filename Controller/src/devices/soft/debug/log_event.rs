use crate::{
    devices,
    signals::{self, signal, types::event::Value},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use std::{any::type_name, borrow::Cow};

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub name: String,
}

#[derive(Debug)]
pub struct Device<V>
where
    V: Value + Clone,
{
    configuration: Configuration,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signal_input: signal::event_target_queued::Signal<V>,
}
impl<V> Device<V>
where
    V: Value + Clone,
{
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signal_input: signal::event_target_queued::Signal::<V>::new(),
        }
    }

    fn signals_targets_changed(&self) {
        let values = self.signal_input.take_pending();
        for value in values.into_vec().into_iter() {
            log::info!("{}: {:?}", self.configuration.name, value);
        }
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        futures::stream::once(async {})
            .chain(self.signals_targets_changed_waker.stream())
            .stream_take_until_exhausted(exit_flag)
            .for_each(async move |()| {
                self.signals_targets_changed();
            })
            .await;

        Exited
    }
}

impl<V> devices::Device for Device<V>
where
    V: Value + Clone,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!("soft/debug/log_event<{}>", type_name::<V>()))
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
        self.run(exit_flag).await
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SignalIdentifier {
    Input,
}
impl signals::Identifier for SignalIdentifier {}
impl<V> signals::Device for Device<V>
where
    V: Value + Clone,
{
    fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
        Some(&self.signals_targets_changed_waker)
    }
    fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
        None
    }

    type Identifier = SignalIdentifier;
    fn by_identifier(&self) -> signals::ByIdentifier<Self::Identifier> {
        hashmap! {
            SignalIdentifier::Input => &self.signal_input as &dyn signal::Base,
        }
    }
}
