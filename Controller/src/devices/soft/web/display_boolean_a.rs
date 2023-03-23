use crate::{
    devices,
    signals::{self, signal},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runtime::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use maplit::hashmap;
use serde::Serialize;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Device {
    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signal_input: signal::state_target_last::Signal<bool>,

    gui_summary_waker: devices::gui_summary::Waker,
}
impl Device {
    pub fn new() -> Self {
        Self {
            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signal_input: signal::state_target_last::Signal::<bool>::new(),

            gui_summary_waker: devices::gui_summary::Waker::new(),
        }
    }

    fn signals_targets_changed(&self) {
        let mut gui_summary_changed = false;

        if self.signal_input.take_pending().is_some() {
            // we don't really care about the value, as it's going to be read by gui summary value
            gui_summary_changed = true;
        }

        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.signals_targets_changed_waker
            .stream()
            .stream_take_until_exhausted(exit_flag)
            .for_each(async move |()| {
                self.signals_targets_changed();
            })
            .await;

        Exited
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/web/display_boolean_a")
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase {
        self
    }
    fn as_gui_summary_device_base(&self) -> Option<&dyn devices::gui_summary::DeviceBase> {
        Some(self)
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
}
impl signals::Identifier for SignalIdentifier {}
impl signals::Device for Device {
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

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct GuiSummary {
    value: Option<bool>,
}
impl devices::gui_summary::Device for Device {
    fn waker(&self) -> &devices::gui_summary::Waker {
        &self.gui_summary_waker
    }

    type Value = GuiSummary;
    fn value(&self) -> Self::Value {
        let value = self.signal_input.peek_last();

        Self::Value { value }
    }
}
