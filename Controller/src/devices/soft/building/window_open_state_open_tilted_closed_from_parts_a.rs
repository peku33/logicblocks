use crate::{
    datatypes::building::window::WindowOpenStateOpenTiltedClosed,
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
pub struct Configuration {
    // whether to treat opened = true, tilted = false as "Open" (true) or
    // "Unknown" (false). This could be useful not to go to Unknown state if
    // there is a race condition between sensors
    pub open_on_opened_not_tilted: bool,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input_opened: signal::state_target_last::Signal<bool>,
    signal_input_tilted: signal::state_target_last::Signal<bool>,
    signal_output: signal::state_source::Signal<WindowOpenStateOpenTiltedClosed>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input_opened: signal::state_target_last::Signal::<bool>::new(),
            signal_input_tilted: signal::state_target_last::Signal::<bool>::new(),
            signal_output: signal::state_source::Signal::<WindowOpenStateOpenTiltedClosed>::new(
                None,
            ),
        }
    }

    fn calculate(
        &self,
        input_opened: Option<bool>,
        input_tilted: Option<bool>,
    ) -> Option<WindowOpenStateOpenTiltedClosed> {
        match (input_opened, input_tilted) {
            (Some(input_opened), Some(input_tilted)) => match (input_opened, input_tilted) {
                (false, false) => Some(WindowOpenStateOpenTiltedClosed::Closed),
                (false, true) => Some(WindowOpenStateOpenTiltedClosed::Tilted),
                (true, true) => Some(WindowOpenStateOpenTiltedClosed::Open),
                (true, false) => {
                    if self.configuration.open_on_opened_not_tilted {
                        Some(WindowOpenStateOpenTiltedClosed::Open)
                    } else {
                        None
                    }
                }
            },
            _ => None,
        }
    }

    fn signals_targets_changed(&self) {
        let signal_output = self.calculate(
            self.signal_input_opened.take_last().value,
            self.signal_input_tilted.take_last().value,
        );

        if self.signal_output.set_one(signal_output) {
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
        Cow::from("soft/building/window_open_state_open_tilted_closed_from_parts_a")
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
    InputOpened,
    InputTilted,
    Output,
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
            SignalIdentifier::InputOpened => &self.signal_input_opened as &dyn signal::Base,
            SignalIdentifier::InputTilted => &self.signal_input_tilted as &dyn signal::Base,
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}
