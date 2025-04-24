use crate::{
    datatypes::real::Real,
    devices,
    signals::{self, signal, types::state::Value},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::StreamExt;
use maplit::hashmap;
use std::{any::type_name, borrow::Cow};

#[derive(Debug)]
pub struct Device<V>
where
    V: Value + Clone,
    Real: TryFrom<V>,
{
    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input: signal::state_target_queued::Signal<V>,
    signal_output: signal::state_source::Signal<Real>,
}
impl<V> Device<V>
where
    V: Value + Clone,
    Real: TryFrom<V>,
{
    pub fn new() -> Self {
        Self {
            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::state_target_queued::Signal::<V>::new(),
            signal_output: signal::state_source::Signal::<Real>::new(None),
        }
    }

    fn signals_targets_changed(&self) {
        let values = self
            .signal_input
            .take_pending()
            .into_iter()
            .map(|value| value.and_then(|value| Real::try_from(value).ok()))
            .collect::<Box<[Option<Real>]>>();

        if self.signal_output.set_many(values) {
            self.signals_sources_changed_waker.wake();
        }
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.signals_targets_changed_waker.stream()
            .stream_take_until_exhausted(exit_flag)
            .for_each(async |()| {
                self.signals_targets_changed();
            })
            .await;

        Exited
    }
}

impl<V> devices::Device for Device<V>
where
    V: Value + Clone,
    Real: TryFrom<V>,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!(
            "soft/converter/real_to_state_a<{}>",
            type_name::<V>()
        ))
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
    Real: TryFrom<V>,
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
    Output,
}
impl signals::Identifier for SignalIdentifier {}
impl<V> signals::Device for Device<V>
where
    V: Value + Clone,
    Real: TryFrom<V>,
{
    fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
        Some(&self.signals_targets_changed_waker)
    }
    fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
        Some(&self.signals_sources_changed_waker)
    }

    type Identifier = SignalIdentifier;
    fn by_identifier(&self) -> signals::ByIdentifier<Self::Identifier> {
        hashmap! {
            SignalIdentifier::Input => &self.signal_input as &dyn signal::Base,
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}
