use crate::{
    devices,
    signals::{
        self, signal,
        types::{event::Value as EventValue, state::Value as StateValue},
    },
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use maplit::hashmap;
use std::{any::type_name, borrow::Cow};

#[derive(Debug)]
pub struct Configuration<V>
where
    V: EventValue + StateValue + Clone,
{
    /// Value emitted on output before any event is received. `None` leaves the
    /// output unset until the first event.
    pub initial: Option<V>,
}

/// Event<V> is provided on input target
/// last received value is kept as State<V> on output source
#[derive(Debug)]
pub struct Device<V>
where
    V: EventValue + StateValue + Clone,
{
    configuration: Configuration<V>,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input: signal::event_target_last::Signal<V>,
    signal_output: signal::state_source::Signal<V>,
}
impl<V> Device<V>
where
    V: EventValue + StateValue + Clone,
{
    pub fn new(configuration: Configuration<V>) -> Self {
        let initial = configuration.initial.clone();

        Self {
            configuration,
            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::event_target_last::Signal::<V>::new(),
            signal_output: signal::state_source::Signal::<V>::new(initial),
        }
    }

    fn signals_targets_changed(&self) {
        // act only if an event was received
        let input = match self.signal_input.take_pending() {
            Some(input) => input,
            None => return,
        };

        let output = Some(input);

        if self.signal_output.set_one(output) {
            self.signals_sources_changed_waker.wake();
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

impl<V> devices::Device for Device<V>
where
    V: EventValue + StateValue + Clone,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!("soft/value/last_a<{}>", type_name::<V>()))
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
    fn by_identifier(&self) -> signals::ByIdentifier<'_, Self::Identifier> {
        hashmap! {
            SignalIdentifier::Input => &self.signal_input as &dyn signal::Base,
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}
