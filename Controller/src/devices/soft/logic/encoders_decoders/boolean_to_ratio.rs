use crate::{
    datatypes::ratio::Ratio,
    devices,
    signals::{self, signal},
    util::waker_stream,
};
use std::borrow::Cow;

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
            .count();
        let counts_one = inputs_values
            .iter()
            .filter(|last| last.value.contains(&true))
            .count();

        let ratio = (counts_one as f64) / (counts_known as f64);
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

    fn as_signals_device(&self) -> &dyn signals::Device {
        self
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
        std::iter::empty()
            .chain([
                (0, &self.signal_output as &dyn signal::Base), // 0 - output
            ])
            .chain(
                // 1 + n - inputs
                self.signals_input
                    .iter()
                    .enumerate()
                    .map(|(input_id, signal_input)| {
                        (
                            (1 + input_id) as signals::Id,
                            signal_input as &dyn signal::Base,
                        )
                    }),
            )
            .collect::<signals::Signals>()
    }
}
