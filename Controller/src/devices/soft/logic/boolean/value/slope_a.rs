use crate::{
    devices,
    signals::{self, signal},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use maplit::hashmap;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Device {
    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input: signal::state_target_queued::Signal<bool>,
    signal_output_raising: signal::event_source::Signal<()>,
    signal_output_falling: signal::event_source::Signal<()>,
    signal_output_raising_or_falling: signal::event_source::Signal<()>,
}
impl Device {
    pub fn new() -> Self {
        Self {
            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::state_target_queued::Signal::<bool>::new(),
            signal_output_raising: signal::event_source::Signal::<()>::new(),
            signal_output_falling: signal::event_source::Signal::<()>::new(),
            signal_output_raising_or_falling: signal::event_source::Signal::<()>::new(),
        }
    }

    fn signals_targets_changed(&self) {
        let mut raising = 0;
        let mut falling = 0;

        for value in self.signal_input.take_pending().into_iter().flatten() {
            if value {
                raising += 1;
            } else {
                falling += 1;
            }
        }

        let mut signals_sources_changed = false;

        for _ in 0..raising {
            if self.signal_output_raising.push_one(()) {
                signals_sources_changed = true;
            }
        }
        for _ in 0..falling {
            if self.signal_output_falling.push_one(()) {
                signals_sources_changed = true;
            }
        }
        for _ in 0..(raising + falling) {
            if self.signal_output_raising_or_falling.push_one(()) {
                signals_sources_changed = true;
            }
        }

        if signals_sources_changed {
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

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/logic/boolean/value/slope_a")
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase {
        self
    }
}

#[async_trait]
impl Runnable for Device {
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
    OutputRaising,
    OutputFalling,
    OutputRaisingOrFalling,
}
impl signals::Identifier for SignalIdentifier {}
impl signals::Device for Device {
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
            SignalIdentifier::OutputRaising => &self.signal_output_raising as &dyn signal::Base,
            SignalIdentifier::OutputFalling => &self.signal_output_falling as &dyn signal::Base,
            SignalIdentifier::OutputRaisingOrFalling => &self.signal_output_raising_or_falling as &dyn signal::Base,
        }
    }
}
