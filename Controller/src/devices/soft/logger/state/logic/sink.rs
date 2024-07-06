use super::super::hardware::{sink::SinkTypedRef, types::Type};
use crate::{
    devices,
    signals::{self, signal, types::state::Value},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use chrono::Utc;
use futures::stream::StreamExt;
use maplit::hashmap;
use std::{any::type_name, borrow::Cow};

#[derive(Debug)]
pub struct Device<'a, V>
where
    V: Value + Type + Clone,
{
    sink: SinkTypedRef<'a, V>,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signal_input: signal::state_target_queued::Signal<V>,
}
impl<'a, V> Device<'a, V>
where
    V: Value + Type + Clone,
{
    pub fn new(sink: SinkTypedRef<'a, V>) -> Self {
        Self {
            sink,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signal_input: signal::state_target_queued::Signal::<V>::new(),
        }
    }

    fn signals_targets_changed(&self) {
        let now = Utc::now();

        self.signal_input
            .take_pending()
            .into_vec()
            .into_iter()
            .for_each(|value| {
                self.sink.push(now, value);
            });
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

impl<'a, V> devices::Device for Device<'a, V>
where
    V: Value + Type + Clone,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!("soft/logger/state/sink<{}>", type_name::<V>()))
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase {
        self
    }
}

#[async_trait]
impl<'a, V> Runnable for Device<'a, V>
where
    V: Value + Type + Clone,
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
impl<'a, V> signals::Device for Device<'a, V>
where
    V: Value + Type + Clone,
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
