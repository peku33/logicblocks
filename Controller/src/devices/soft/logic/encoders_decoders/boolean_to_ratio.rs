use crate::{
    datatypes::ratio::Ratio,
    devices,
    signals::{self, signal},
    util::{
        async_flag,
        runtime::{Exited, Runnable},
        waker_stream,
    },
    web::uri_cursor,
};
use async_trait::async_trait;
use std::{borrow::Cow, convert::TryFrom};

#[derive(Debug)]
pub struct Configuration {
    pub inputs_count: usize,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_output: signal::state_source::Signal<Ratio>,
    signals_input: Vec<signal::state_target_last::Signal<bool>>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        let inputs_count = configuration.inputs_count;

        Self {
            configuration,

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_output: signal::state_source::Signal::<Ratio>::new(None),
            signals_input: (0..inputs_count)
                .map(|_input_id| signal::state_target_last::Signal::<bool>::new())
                .collect::<Vec<_>>(),
        }
    }

    fn recalculate(&self) {
        let inputs_values = self
            .signals_input
            .iter()
            .map(|signal_input| signal_input.take_last())
            .collect::<Vec<_>>();

        // if no signal is pending, don't recalculate
        if !inputs_values.iter().any(|value| value.pending) {
            return;
        }

        let counts_known = inputs_values
            .iter()
            .filter(|last| last.value.is_some())
            .count() as f64;
        let counts_one = inputs_values
            .iter()
            .filter(|last| last.value.contains(&true))
            .count() as f64;

        let ratio = counts_one / counts_known;
        let ratio: Option<Ratio> = if ratio.is_finite() {
            Some(Ratio::try_from(ratio).unwrap())
        } else {
            // eg. division by zero
            None
        };

        if self.signal_output.set_one(ratio) {
            self.signal_sources_changed_waker.wake();
        }
    }
}
impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/logic/encoders_decoders/boolean_to_ratio")
    }

    fn as_runnable(&self) -> &dyn Runnable {
        self
    }
    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
    fn as_gui_summary_provider(&self) -> Option<&dyn devices::GuiSummaryProvider> {
        None
    }
    fn as_web_handler(&self) -> Option<&dyn uri_cursor::Handler> {
        None
    }
}
#[async_trait]
impl Runnable for Device {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        exit_flag.await;
        Exited
    }
}
impl signals::Device for Device {
    fn signal_targets_changed_wake(&self) {
        self.recalculate()
    }

    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }

    fn signals(&self) -> signals::Signals {
        // 0 - output
        // 1 + n - inputs

        let signal_output_iter = std::array::IntoIter::new([
            (0u16, &self.signal_output as &dyn signal::Base), // line break
        ]);
        let signals_input_iter =
            self.signals_input
                .iter()
                .enumerate()
                .map(|(input_id, signal_input)| {
                    ((1 + input_id) as u16, signal_input as &dyn signal::Base)
                });

        std::iter::empty()
            .chain(signal_output_iter)
            .chain(signals_input_iter)
            .collect::<signals::Signals>()
    }
}
