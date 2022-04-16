use crate::{
    devices,
    signals::{self, signal},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runtime::{Exited, Runnable},
        waker_stream,
    },
    web::{self, uri_cursor},
};
use async_trait::async_trait;
use futures::{
    future::{BoxFuture, FutureExt},
    stream::StreamExt,
};
use maplit::hashmap;
use parking_lot::RwLock;
use serde::Serialize;
use std::borrow::Cow;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    PassThrough,
    Override(bool),
}

#[derive(Debug)]
pub struct Configuration {
    pub initial_mode: Mode,
}

#[derive(Debug)]
enum RecalculateOperation {
    None,
    ModeSet(Mode),
    ModeCycle,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,
    mode: RwLock<Mode>,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input: signal::state_target_last::Signal<bool>,
    signal_mode_set_pass_through: signal::event_target_last::Signal<()>,
    signal_mode_set_override: signal::event_target_last::Signal<bool>,
    signal_mode_cycle: signal::event_target_last::Signal<()>,
    signal_output: signal::state_source::Signal<bool>,

    gui_summary_waker: waker_stream::mpmc::Sender,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        let mode = configuration.initial_mode;

        let initial_value = match mode {
            Mode::Override(value) => Some(value),
            Mode::PassThrough => None,
        };

        Self {
            configuration,
            mode: RwLock::new(mode),

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::state_target_last::Signal::<bool>::new(),
            signal_mode_set_pass_through: signal::event_target_last::Signal::<()>::new(),
            signal_mode_set_override: signal::event_target_last::Signal::<bool>::new(),
            signal_mode_cycle: signal::event_target_last::Signal::<()>::new(),
            signal_output: signal::state_source::Signal::<bool>::new(initial_value),

            gui_summary_waker: waker_stream::mpmc::Sender::new(),
        }
    }

    fn mode_cycle_next(
        mode: Mode,
        input_value: Option<bool>,
    ) -> Mode {
        match (mode, input_value) {
            (Mode::PassThrough, Some(input_value)) => Mode::Override(!input_value),
            (Mode::PassThrough, None) => Mode::Override(false),
            (Mode::Override(override_value), Some(input_value)) => {
                if override_value == input_value {
                    Mode::Override(!override_value)
                } else {
                    Mode::PassThrough
                }
            }
            (Mode::Override(override_value), None) => Mode::Override(!override_value),
        }
    }

    fn signals_targets_changed(&self) {
        let recalculate_operation = match (
            self.signal_mode_set_pass_through.take_pending(),
            self.signal_mode_set_override.take_pending(),
            self.signal_mode_cycle.take_pending(),
        ) {
            (Some(()), None, None) => RecalculateOperation::ModeSet(Mode::PassThrough),
            (None, Some(override_value), None) => {
                RecalculateOperation::ModeSet(Mode::Override(override_value))
            }
            (None, None, Some(())) => RecalculateOperation::ModeCycle,
            _ => RecalculateOperation::None, // input could have been changed
        };

        self.recalculate(recalculate_operation);
    }

    fn recalculate(
        &self,
        operation: RecalculateOperation,
    ) {
        let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        let mut mode_lock = self.mode.write();

        let signal::state_target_last::Last::<bool> {
            value: input_value,
            pending: input_pending,
        } = self.signal_input.take_last();
        if input_pending {
            gui_summary_changed = true;
        }

        match operation {
            RecalculateOperation::None => {}
            RecalculateOperation::ModeSet(mode) => {
                if *mode_lock != mode {
                    *mode_lock = mode;
                    gui_summary_changed = true;
                }
            }
            RecalculateOperation::ModeCycle => {
                let mode = Self::mode_cycle_next(*mode_lock, input_value);
                if *mode_lock != mode {
                    *mode_lock = mode;
                    gui_summary_changed = true;
                }
            }
        }

        let mode = *mode_lock;
        drop(mode_lock);

        let output_value = match (mode, input_value) {
            (Mode::PassThrough, input_value) => input_value,
            (Mode::Override(override_value), _) => Some(override_value),
        };
        if self.signal_output.set_one(output_value) {
            signal_sources_changed = true;
        }

        if signal_sources_changed {
            self.signals_sources_changed_waker.wake();
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
            .stream(false)
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
        Cow::from("soft/logic/boolean/flip_flop/override_a")
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device_base(&self) -> &dyn signals::DeviceBase {
        self
    }
    fn as_gui_summary_provider(&self) -> Option<&dyn devices::GuiSummaryProvider> {
        Some(self)
    }
    fn as_web_handler(&self) -> Option<&dyn uri_cursor::Handler> {
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
    ModeSetPassThrough,
    ModeSetOverride,
    ModeCycle,
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
            SignalIdentifier::Input => &self.signal_input as &dyn signal::Base,
            SignalIdentifier::ModeSetPassThrough => &self.signal_mode_set_pass_through as &dyn signal::Base,
            SignalIdentifier::ModeSetOverride => &self.signal_mode_set_override as &dyn signal::Base,
            SignalIdentifier::ModeCycle => &self.signal_mode_cycle as &dyn signal::Base,
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "mode")]
enum GuiSummaryMode {
    PassThrough,
    Override { value: bool },
}
#[derive(Serialize)]
struct GuiSummary {
    input_value: Option<bool>,
    mode: GuiSummaryMode,
}
impl devices::GuiSummaryProvider for Device {
    fn value(&self) -> Box<dyn devices::GuiSummary> {
        let input_value = self.signal_input.peek_last();
        let mode = *self.mode.read();

        let gui_summary_mode = match mode {
            Mode::PassThrough => GuiSummaryMode::PassThrough,
            Mode::Override(value) => GuiSummaryMode::Override { value },
        };
        let gui_summary = GuiSummary {
            input_value,
            mode: gui_summary_mode,
        };
        let gui_summary = Box::new(gui_summary);
        gui_summary
    }
    fn waker(&self) -> waker_stream::mpmc::ReceiverFactory {
        self.gui_summary_waker.receiver_factory()
    }
}

impl uri_cursor::Handler for Device {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Next("mode", uri_cursor) => match uri_cursor.as_ref() {
                uri_cursor::UriCursor::Next("set", uri_cursor) => match uri_cursor.as_ref() {
                    uri_cursor::UriCursor::Terminal => match *request.method() {
                        http::Method::POST => {
                            let value = match request.body_parse_json::<Option<bool>>() {
                                Ok(value) => value,
                                Err(error) => return async move {
                                    web::Response::error_400_from_error(error)
                                }
                                .boxed(),
                            };

                            let recalculate_operation = match value {
                                Some(override_value) => {
                                    RecalculateOperation::ModeSet(Mode::Override(override_value))
                                }
                                None => RecalculateOperation::ModeSet(Mode::PassThrough),
                            };

                            self.recalculate(recalculate_operation);

                            async move { web::Response::ok_empty() }.boxed()
                        }
                        _ => async move { web::Response::error_405() }.boxed(),
                    },
                    _ => async move { web::Response::error_404() }.boxed(),
                },
                uri_cursor::UriCursor::Next("cycle", uri_cursor) => match uri_cursor.as_ref() {
                    uri_cursor::UriCursor::Terminal => match *request.method() {
                        http::Method::POST => {
                            self.recalculate(RecalculateOperation::ModeCycle);

                            async move { web::Response::ok_empty() }.boxed()
                        }
                        _ => async move { web::Response::error_405() }.boxed(),
                    },
                    _ => async move { web::Response::error_404() }.boxed(),
                },
                _ => async move { web::Response::error_404() }.boxed(),
            },
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
