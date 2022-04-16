use crate::{
    devices,
    signals::{
        self, signal,
        types::{event::Value as EventValue, state::Value as StateValue},
    },
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runtime::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use maplit::hashmap;
use std::{any::type_name, borrow::Cow};

/**
 * State<V> signal is provided on input target
 * when Event<()> hits trigger target
 * current input value is emitted as Event<V> from output source
 */
#[derive(Debug)]
pub struct Device<V>
where
    V: EventValue + StateValue + Clone,
{
    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input: signal::state_target_last::Signal<V>,
    signal_trigger: signal::event_target_last::Signal<()>,
    signal_output: signal::event_source::Signal<V>,
}
impl<V> Device<V>
where
    V: EventValue + StateValue + Clone,
{
    pub fn new() -> Self {
        Self {
            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::state_target_last::Signal::<V>::new(),
            signal_trigger: signal::event_target_last::Signal::<()>::new(),
            signal_output: signal::event_source::Signal::<V>::new(),
        }
    }

    fn signals_targets_changed(&self) {
        let mut signal_sources_changed = false;

        if let Some(()) = self.signal_trigger.take_pending() {
            let value = self.signal_input.take_last().value;
            if let Some(value) = value {
                if self.signal_output.push_one(value) {
                    signal_sources_changed = true;
                }
            }
        }

        if signal_sources_changed {
            self.signals_sources_changed_waker.wake();
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

impl<V> devices::Device for Device<V>
where
    V: EventValue + StateValue + Clone,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!("soft/value/sample_a<{}>", type_name::<V>()))
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
    V: EventValue + StateValue + Clone,
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
    Trigger,
    Output,
}
impl signals::Identifier for SignalIdentifier {}
impl<V> signals::Device for Device<V>
where
    V: EventValue + StateValue + Clone,
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
            SignalIdentifier::Trigger => &self.signal_trigger as &dyn signal::Base,
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}
