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
use futures::stream::StreamExt;
use maplit::hashmap;
use serde::Serialize;
use std::{borrow::Cow, fmt};

pub trait Specification: Send + Sync + fmt::Debug + 'static {
    type Type: Value + Clone + Serialize;

    fn name() -> Cow<'static, str>;
}

#[derive(Debug)]
pub struct Device<S>
where
    S: Specification,
{
    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signal_input: signal::state_target_last::Signal<S::Type>,

    gui_summary_waker: devices::gui_summary::Waker,
}
impl<S> Device<S>
where
    S: Specification,
{
    pub fn new() -> Self {
        Self {
            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signal_input: signal::state_target_last::Signal::<S::Type>::new(),

            gui_summary_waker: devices::gui_summary::Waker::new(),
        }
    }

    fn signals_targets_changed(&self) {
        if self.signal_input.take_pending().is_some() {
            // we don't really care about the value, as it's going to be read by gui summary
            // value
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
            .for_each(async |()| {
                self.signals_targets_changed();
            })
            .await;

        Exited
    }
}

impl<S> devices::Device for Device<S>
where
    S: Specification,
{
    fn class(&self) -> Cow<'static, str> {
        Cow::from(format!("soft/web/display/{}", S::name()))
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
impl<S> Runnable for Device<S>
where
    S: Specification,
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
impl<S> signals::Device for Device<S>
where
    S: Specification,
{
    fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
        Some(&self.signals_targets_changed_waker)
    }
    fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
        None
    }

    type Identifier = SignalIdentifier;
    fn by_identifier(&self) -> signals::ByIdentifier<'_, Self::Identifier> {
        hashmap! {
            SignalIdentifier::Input => &self.signal_input as &dyn signal::Base,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct GuiSummary<S>
where
    S: Specification,
{
    value: Option<S::Type>,
}
impl<S> devices::gui_summary::Device for Device<S>
where
    S: Specification,
{
    fn waker(&self) -> &devices::gui_summary::Waker {
        &self.gui_summary_waker
    }

    type Value = GuiSummary<S>;
    fn value(&self) -> Self::Value {
        let value = self.signal_input.peek_last();

        Self::Value { value }
    }
}
