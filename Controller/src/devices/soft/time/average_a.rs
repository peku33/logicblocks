use crate::{
    datatypes::real::Real,
    devices,
    signals::{self, signal},
    util::{
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use anyhow::{Error, ensure};
use async_trait::async_trait;
use futures::{
    FutureExt,
    future::{Fuse, FusedFuture},
    pin_mut, select,
    stream::StreamExt,
};
use maplit::hashmap;
use std::{
    borrow::Cow,
    collections::LinkedList,
    iter,
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct Configuration {
    pub window: Duration,
    pub tick_duration_min: Duration,
    pub tick_duration_max: Duration,
}
impl Configuration {
    pub fn validate(&self) -> Result<(), Error> {
        ensure!(self.tick_duration_min < self.tick_duration_max);

        Ok(())
    }
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input: signal::state_target_last::Signal<Real>,
    signal_output: signal::state_source::Signal<Real>,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        configuration.validate().unwrap();

        Self {
            configuration,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input: signal::state_target_last::Signal::<Real>::new(),
            signal_output: signal::state_source::Signal::<Real>::new(None),
        }
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let signals_targets_changed_stream = self.signals_targets_changed_waker.stream();
        pin_mut!(signals_targets_changed_stream);

        let mut time_average = TimeWeightedAverage::new(self.configuration.window);

        // will perform tick on first iteration
        let tick_next_min = Fuse::<tokio::time::Sleep>::terminated();
        pin_mut!(tick_next_min);

        // will be set on first iteration
        let tick_next_max = Fuse::<tokio::time::Sleep>::terminated();
        pin_mut!(tick_next_max);

        loop {
            let now = Instant::now();

            // if new value is available - put it in the queue
            if let Some(input) = self.signal_input.take_pending() {
                let input = input.map(|input| input.to_f64());

                time_average.push(now, input);
            }

            if tick_next_min.is_terminated() {
                // enough time has passed, so we can perform the tick
                let output = time_average.tick(now);
                let output = output.map(|output| Real::from_f64(output).unwrap());

                if self.signal_output.set_one(output) {
                    self.signals_sources_changed_waker.wake();
                }

                // restart min and max timers
                tick_next_min.set(
                    tokio::time::sleep_until(tokio::time::Instant::from_std(
                        now + self.configuration.tick_duration_min,
                    ))
                    .fuse(),
                );
                tick_next_max.set(
                    tokio::time::sleep_until(tokio::time::Instant::from_std(
                        now + self.configuration.tick_duration_max,
                    ))
                    .fuse(),
                );
            }

            select! {
                () = signals_targets_changed_stream.select_next_some() => {},
                () = &mut tick_next_min => {},
                () = &mut tick_next_max => {},
                () = exit_flag => break,
            }
        }

        Exited
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/time/average_a")
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
            SignalIdentifier::Input => &self.signal_input as &dyn signal::Base,
            SignalIdentifier::Output => &self.signal_output as &dyn signal::Base,
        }
    }
}

#[derive(Debug)]
struct TimeWeightedAverage {
    window: Duration,

    history: LinkedList<(Instant, Option<f64>)>,
}
impl TimeWeightedAverage {
    pub fn new(window: Duration) -> Self {
        let history = LinkedList::<(Instant, Option<f64>)>::new();

        Self { window, history }
    }

    pub fn push(
        &mut self,
        now: Instant,
        value: Option<f64>,
    ) {
        // the history must either be empty or the last value must be before now
        debug_assert!(
            self.history
                .back()
                .map(|(last, _)| *last <= now)
                .unwrap_or(true)
        );

        // insert the element
        self.history.push_back((now, value));
    }

    pub fn tick(
        &mut self,
        now: Instant,
    ) -> Option<f64> {
        let time_cutoff = now - self.window;

        let mut cutoff_index = 0usize;
        let mut weight_total = 0.0f64;
        let mut value_weighted_total = 0.0f64;

        self.history
            .iter()
            .chain(iter::once(&(now, None))) // virtually append (now, None)
            .map_windows(|[(time_start, value), (time_end, _)]| (*time_start, *time_end, *value))
            .enumerate()
            .for_each(|(index, (time_start, time_end, value))| {
                debug_assert!(time_start <= time_end);

                // mark stale items (all outside usable range) for removal
                if time_end <= time_cutoff {
                    cutoff_index = index + 1;
                    return;
                }

                // skip empty items
                let value = match value {
                    Some(value) => value,
                    None => return,
                };

                // adjust time_start to usable portion (if part of time_start lays before
                // time_cutoff)
                let time_start = time_start.max(time_cutoff);
                debug_assert!(time_start <= time_end);

                // use f64 seconds as weight
                let weight = (time_end - time_start).as_secs_f64();

                // update global counters
                weight_total += weight;
                value_weighted_total += value * weight;
            });

        // calculate final value
        let value_average = if weight_total > 0.0 {
            Some(value_weighted_total / weight_total)
        } else {
            None
        };

        // remove stale items
        for _ in 0..cutoff_index {
            self.history.pop_front();
        }

        value_average
    }
}
#[cfg(test)]
mod tests_time_weighted_average {
    use super::TimeWeightedAverage;
    use std::time::{Duration, Instant};

    #[test]
    fn empty() {
        let window = Duration::from_secs(5);
        let mut time_weighted_average = TimeWeightedAverage::new(window);

        assert!(time_weighted_average.tick(Instant::now()).is_none());
    }

    #[test]
    fn single() {
        let window = Duration::from_secs(5);
        let mut time_weighted_average = TimeWeightedAverage::new(window);

        // insert a value at now marker
        let now = Instant::now();
        time_weighted_average.push(now, Some(100.0));

        // in the same time as added - it should be zero
        assert!(time_weighted_average.tick(now).is_none());

        // one second later - it should be equal to value
        assert!(
            time_weighted_average
                .tick(now + Duration::from_secs(1))
                .unwrap()
                == 100.0
        );

        // one second before window time it should also be the same
        assert!(
            time_weighted_average
                .tick(now + Duration::from_secs(4))
                .unwrap()
                == 100.0
        );

        // exactly at window border it should still be the same
        assert!(
            time_weighted_average
                .tick(now + Duration::from_secs(5))
                .unwrap()
                == 100.0
        );

        // after window closes - it should again be zero
        assert!(time_weighted_average.tick(now).is_none());
    }

    #[test]
    fn sequence_1() {
        let window = Duration::from_secs(5);
        let mut time_weighted_average = TimeWeightedAverage::new(window);
        let now = Instant::now();

        time_weighted_average.push(now + Duration::from_secs(0), Some(150.0));
        time_weighted_average.push(now + Duration::from_secs(2), None);
        time_weighted_average.push(now + Duration::from_secs(3), Some(225.0));
        time_weighted_average.push(now + Duration::from_secs(5), None);
        assert!(time_weighted_average.history.len() == 4);

        assert!(
            time_weighted_average
                .tick(now + Duration::from_secs(5))
                .unwrap()
                == 187.5
        );
        assert!(
            time_weighted_average
                .tick(now + Duration::from_secs(6))
                .unwrap()
                == 200.0
        );
        assert!(
            time_weighted_average
                .tick(now + Duration::from_secs(7))
                .unwrap()
                == 225.0
        );
        assert!(time_weighted_average.history.len() == 3);

        assert!(
            time_weighted_average
                .tick(now + Duration::from_secs(8))
                .unwrap()
                == 225.0
        );
        assert!(time_weighted_average.history.len() == 2);

        assert!(
            time_weighted_average
                .tick(now + Duration::from_secs(10))
                .is_none()
        );
        assert!(time_weighted_average.history.len() == 1);
    }
}
