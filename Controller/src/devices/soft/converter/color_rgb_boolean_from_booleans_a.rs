use crate::{
    datatypes::color_rgb_boolean::ColorRgbBoolean,
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
    signal_r: signal::state_target_last::Signal<bool>,
    signal_g: signal::state_target_last::Signal<bool>,
    signal_b: signal::state_target_last::Signal<bool>,
    signal_output: signal::state_source::Signal<ColorRgbBoolean>,
}
impl Device {
    pub fn new() -> Self {
        Self {
            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_r: signal::state_target_last::Signal::<bool>::new(),
            signal_g: signal::state_target_last::Signal::<bool>::new(),
            signal_b: signal::state_target_last::Signal::<bool>::new(),
            signal_output: signal::state_source::Signal::<ColorRgbBoolean>::new(Some(
                ColorRgbBoolean::off(),
            )),
        }
    }

    fn signals_targets_changed(&self) {
        let r = self.signal_r.take_last().value.unwrap_or(false);
        let g = self.signal_g.take_last().value.unwrap_or(false);
        let b = self.signal_b.take_last().value.unwrap_or(false);

        let output = ColorRgbBoolean { r, g, b };
        if self.signal_output.set_one(Some(output)) {
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
        Cow::from("soft/converter/color_rgb_boolean_from_booleans_a")
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
    R,
    G,
    B,
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
    fn by_identifier(&self) -> signals::ByIdentifier<'_, Self::Identifier> {
        hashmap! {
            SignalIdentifier::R => &self.signal_r as &dyn signal::Base,
            SignalIdentifier::G => &self.signal_g as &dyn signal::Base,
            SignalIdentifier::B => &self.signal_b as &dyn signal::Base,
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}
