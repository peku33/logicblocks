use crate::{
    devices,
    signals::{self, signal},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
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
    ModeCyclePassThrough,
    ModeCycleNoPassThrough,
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
    signal_mode_cycle_pass_through: signal::event_target_last::Signal<()>,
    signal_mode_cycle_no_pass_through: signal::event_target_last::Signal<()>,
    signal_output: signal::state_source::Signal<bool>,

    gui_summary_waker: devices::gui_summary::Waker,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        let mode = configuration.initial_mode;

        let initial_value = Self::initial_value(mode);

        Self {
            configuration,
            mode: RwLock::new(mode),

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::state_target_last::Signal::<bool>::new(),
            signal_mode_set_pass_through: signal::event_target_last::Signal::<()>::new(),
            signal_mode_set_override: signal::event_target_last::Signal::<bool>::new(),
            signal_mode_cycle_pass_through: signal::event_target_last::Signal::<()>::new(),
            signal_mode_cycle_no_pass_through: signal::event_target_last::Signal::<()>::new(),
            signal_output: signal::state_source::Signal::<bool>::new(initial_value),

            gui_summary_waker: devices::gui_summary::Waker::new(),
        }
    }

    fn initial_value(initial_mode: Mode) -> Option<bool> {
        match initial_mode {
            Mode::Override(value) => Some(value),
            Mode::PassThrough => None,
        }
    }

    fn mode_cycle_pass_through_next(
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
    fn mode_cycle_no_pass_through_next(
        mode: Mode,
        input_value: Option<bool>,
    ) -> Mode {
        match (mode, input_value) {
            (Mode::PassThrough, Some(input_value)) => Mode::Override(!input_value),
            (Mode::PassThrough, None) => Mode::Override(false),
            (Mode::Override(override_value), _) => Mode::Override(!override_value),
        }
    }

    fn signals_targets_changed(&self) {
        let mode_set_pass_through = self.signal_mode_set_pass_through.take_pending() == Some(());
        let mode_set_override = self.signal_mode_set_override.take_pending();
        let mode_cycle_pass_through =
            self.signal_mode_cycle_pass_through.take_pending() == Some(());
        let mode_cycle_no_pass_through =
            self.signal_mode_cycle_no_pass_through.take_pending() == Some(());

        let recalculate_operation = match (
            mode_set_pass_through,
            mode_set_override,
            mode_cycle_pass_through,
            mode_cycle_no_pass_through,
        ) {
            (true, None, false, false) => RecalculateOperation::ModeSet(Mode::PassThrough),
            (false, Some(override_value), false, false) => {
                RecalculateOperation::ModeSet(Mode::Override(override_value))
            }
            (false, None, true, false) => RecalculateOperation::ModeCyclePassThrough,
            (false, None, false, true) => RecalculateOperation::ModeCycleNoPassThrough,
            _ => RecalculateOperation::None, // input could have been changed
        };

        self.recalculate(recalculate_operation);
    }

    fn recalculate(
        &self,
        operation: RecalculateOperation,
    ) {
        let mut signals_sources_changed = false;
        let mut gui_summary_changed = false;

        let mut mode_lock = self.mode.write();

        let signal::state_target_last::Last::<bool> {
            value: input_value,
            pending: input_pending,
        } = self.signal_input.take_last();
        if input_pending {
            gui_summary_changed = true;
        }

        let mode = match operation {
            RecalculateOperation::None => None,
            RecalculateOperation::ModeSet(mode) => Some(mode),
            RecalculateOperation::ModeCyclePassThrough => {
                Some(Self::mode_cycle_pass_through_next(*mode_lock, input_value))
            }
            RecalculateOperation::ModeCycleNoPassThrough => Some(
                Self::mode_cycle_no_pass_through_next(*mode_lock, input_value),
            ),
        };

        if let Some(mode) = mode
            && *mode_lock != mode
        {
            *mode_lock = mode;
            gui_summary_changed = true;
        }

        let mode = *mode_lock;
        drop(mode_lock);

        let output = match (mode, input_value) {
            (Mode::PassThrough, input_value) => input_value,
            (Mode::Override(override_value), _) => Some(override_value),
        };

        signals_sources_changed |= self.signal_output.set_one(output);

        if signals_sources_changed {
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
        Cow::from("soft/logic/boolean/flip_flop/override_a")
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
    ModeCyclePassThrough,
    ModeCycleNoPassThrough,
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
            SignalIdentifier::ModeCyclePassThrough => &self.signal_mode_cycle_pass_through as &dyn signal::Base,
            SignalIdentifier::ModeCycleNoPassThrough => &self.signal_mode_cycle_no_pass_through as &dyn signal::Base,
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "mode")]
enum GuiSummaryMode {
    PassThrough,
    Override { value: bool },
}
#[derive(Debug, Serialize)]
pub struct GuiSummary {
    input_value: Option<bool>,
    mode: GuiSummaryMode,
}
impl devices::gui_summary::Device for Device {
    fn waker(&self) -> &devices::gui_summary::Waker {
        &self.gui_summary_waker
    }

    type Value = GuiSummary;
    fn value(&self) -> Self::Value {
        let input_value = self.signal_input.peek_last();

        let mode = *self.mode.read();
        let mode = match mode {
            Mode::PassThrough => GuiSummaryMode::PassThrough,
            Mode::Override(value) => GuiSummaryMode::Override { value },
        };

        Self::Value { input_value, mode }
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
                                Err(error) => {
                                    return async { web::Response::error_400_from_error(error) }
                                        .boxed();
                                }
                            };

                            let recalculate_operation = match value {
                                Some(override_value) => {
                                    RecalculateOperation::ModeSet(Mode::Override(override_value))
                                }
                                None => RecalculateOperation::ModeSet(Mode::PassThrough),
                            };

                            self.recalculate(recalculate_operation);

                            async { web::Response::ok_empty() }.boxed()
                        }
                        _ => async { web::Response::error_405() }.boxed(),
                    },
                    _ => async { web::Response::error_404() }.boxed(),
                },
                uri_cursor::UriCursor::Next("cycle", uri_cursor) => match uri_cursor.as_ref() {
                    uri_cursor::UriCursor::Next("pass-through", uri_cursor) => {
                        match uri_cursor.as_ref() {
                            uri_cursor::UriCursor::Terminal => match *request.method() {
                                http::Method::POST => {
                                    self.recalculate(RecalculateOperation::ModeCyclePassThrough);

                                    async { web::Response::ok_empty() }.boxed()
                                }
                                _ => async { web::Response::error_405() }.boxed(),
                            },
                            _ => async { web::Response::error_404() }.boxed(),
                        }
                    }
                    uri_cursor::UriCursor::Next("no-pass-through", uri_cursor) => {
                        match uri_cursor.as_ref() {
                            uri_cursor::UriCursor::Terminal => match *request.method() {
                                http::Method::POST => {
                                    self.recalculate(RecalculateOperation::ModeCycleNoPassThrough);

                                    async { web::Response::ok_empty() }.boxed()
                                }
                                _ => async { web::Response::error_405() }.boxed(),
                            },
                            _ => async { web::Response::error_404() }.boxed(),
                        }
                    }
                    _ => async { web::Response::error_404() }.boxed(),
                },
                _ => async { web::Response::error_404() }.boxed(),
            },
            _ => async { web::Response::error_404() }.boxed(),
        }
    }
}
