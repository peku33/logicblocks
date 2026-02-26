use crate::{
    datatypes::ratio::Ratio,
    devices,
    signals::{self, signal},
    util::{
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::{
    future::{Fuse, FutureExt},
    pin_mut, select,
    stream::StreamExt,
};
use maplit::hashmap;
use parking_lot::RwLock;
use serde::Serialize;
use serde_with::{DurationSecondsWithFrac, serde_as};
use std::{
    borrow::Cow,
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct Configuration {
    pub controller: controller::Configuration,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,
    controller: RwLock<controller::Controller>,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_setpoint: signal::state_target_last::Signal<Ratio>,
    signal_down: signal::state_source::Signal<bool>,
    signal_up: signal::state_source::Signal<bool>,

    gui_summary_waker: devices::gui_summary::Waker,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        let controller = controller::Controller::new(configuration.controller, None);
        let controller = RwLock::new(controller);

        Self {
            configuration,
            controller,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_setpoint: signal::state_target_last::Signal::<Ratio>::new(),
            signal_down: signal::state_source::Signal::<bool>::new(Some(false)),
            signal_up: signal::state_source::Signal::<bool>::new(Some(false)),

            gui_summary_waker: devices::gui_summary::Waker::new(),
        }
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let signals_targets_changed_stream = self.signals_targets_changed_waker.stream();
        pin_mut!(signals_targets_changed_stream);

        let tick_timer = Fuse::<tokio::time::Sleep>::terminated();
        pin_mut!(tick_timer);

        loop {
            let now = Instant::now();
            let setpoint = self.signal_setpoint.take_last().value;

            let tick = self.controller.write().tick(now, setpoint);

            // handle output
            let (down, up) = match tick.output {
                Some(direction) => (
                    direction == controller::Direction::Down,
                    direction == controller::Direction::Up,
                ),
                None => (false, false),
            };
            let mut signals_sources_changed = false;
            signals_sources_changed |= self.signal_down.set_one(Some(down));
            signals_sources_changed |= self.signal_up.set_one(Some(up));
            if signals_sources_changed {
                self.signals_sources_changed_waker.wake();
            }

            // handle next
            match tick.next {
                Some(tick_duration) => tick_timer.set(tokio::time::sleep(tick_duration).fuse()),
                None => tick_timer.set(Fuse::<tokio::time::Sleep>::terminated()),
            }

            // every tick possibly changes the state
            // NOTE: maybe cache the state and wake only if it has actually changed?
            self.gui_summary_waker.wake();

            select! {
                () = signals_targets_changed_stream.select_next_some() => {},
                () = tick_timer => {},
                () = exit_flag => break,
            }
        }

        Exited
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/time/up_down_a")
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
    Setpoint,
    Down,
    Up,
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
            SignalIdentifier::Setpoint => &self.signal_setpoint as &dyn signal::Base,
            SignalIdentifier::Down => &self.signal_down as &dyn signal::Base,
            SignalIdentifier::Up => &self.signal_up as &dyn signal::Base,
        }
    }
}

#[serde_as]
#[derive(Debug, Serialize)]
#[serde(tag = "state")]
pub enum GuiSummaryState {
    Uncalibrated,
    Calibrating {
        #[serde_as(as = "DurationSecondsWithFrac<f64>")]
        #[serde(rename = "started_ago_seconds")]
        started_ago: Duration,

        direction: controller::Direction,

        #[serde_as(as = "DurationSecondsWithFrac<f64>")]
        #[serde(rename = "duration_seconds")]
        duration: Duration,
    },
    Stopped {
        position: Ratio,
        uncertainty_relative: Ratio,
    },
    Moving {
        #[serde_as(as = "DurationSecondsWithFrac<f64>")]
        #[serde(rename = "started_ago_seconds")]
        started_ago: Duration,

        started_position: Ratio,

        setpoint: Ratio,

        #[serde_as(as = "DurationSecondsWithFrac<f64>")]
        #[serde(rename = "duration_seconds")]
        duration: Duration,

        direction: controller::Direction,
    },
}

#[derive(Debug, Serialize)]
pub struct GuiSummary {
    state: GuiSummaryState,
}
impl devices::gui_summary::Device for Device {
    fn waker(&self) -> &devices::gui_summary::Waker {
        &self.gui_summary_waker
    }

    type Value = GuiSummary;
    fn value(&self) -> Self::Value {
        let now = Instant::now();

        let state = match self.controller.read().state() {
            controller::State::Uncalibrated => GuiSummaryState::Uncalibrated,
            controller::State::Calibrating(state) => {
                let controller::StateCalibrating {
                    started,
                    direction,
                    duration,
                } = *state;

                let started_ago = now - started;

                GuiSummaryState::Calibrating {
                    started_ago,
                    direction,
                    duration,
                }
            }
            controller::State::Stopped(state) => {
                let controller::StateStopped { position } = *state;

                let controller::Position {
                    position,
                    uncertainty,
                } = position;

                let uncertainty_relative =
                    if self.configuration.controller.position_uncertainty_max != Ratio::zero() {
                        Ratio::from_f64(
                            (uncertainty.to_f64()
                                / self
                                    .configuration
                                    .controller
                                    .position_uncertainty_max
                                    .to_f64())
                            .clamp(0.0, 1.0),
                        )
                        .unwrap()
                    } else {
                        Ratio::zero()
                    };

                GuiSummaryState::Stopped {
                    position,
                    uncertainty_relative,
                }
            }
            controller::State::Moving(state) => {
                let controller::StateMoving {
                    started,
                    started_position,
                    setpoint,
                    duration,
                    direction,
                } = *state;

                let started_ago = now - started;

                let controller::Position {
                    position: started_position,
                    uncertainty: _,
                } = started_position;

                GuiSummaryState::Moving {
                    started_ago,
                    started_position,
                    setpoint,
                    duration,
                    direction,
                }
            }
        };

        GuiSummary { state }
    }
}

pub mod controller {
    use crate::datatypes::{multiplier::Multiplier, ratio::Ratio};
    use anyhow::{Error, ensure};
    use serde::Serialize;
    use std::time::{Duration, Instant};

    #[derive(Clone, Copy, Debug)]
    pub struct Configuration {
        // full travel duration from fully down to fully up / from fully up to
        // fully down. calculate it from the moment the object starts to move to
        // the moment it stops. don't include control delay (eg. dead_up/down)
        // here. Must be greater then zero.
        pub travel_down: Duration,
        pub travel_up: Duration,

        // how long does object not move after providing the control, eg. how
        // long they don't move after signal is powered. this compensates
        // initial constant delay. dead_(up|down) + travel_(up|down) should be
        // equal to time from powering the motor in extreme position to being
        // automatically stopped in another extreme position.
        pub dead_down: Duration,
        pub dead_up: Duration,

        // initial delay before starting the movement. this prevents motor from quickly changing
        // its direction if user changes his mind. if unsure, set to something like 0.5sec.
        pub start_delay: Duration,

        // what precision is considered "good" enough for movement. controller won't try to move
        // until difference between actual value and desired value is bigger then this setting. 1%
        // is probably sane starting point.
        pub position_offset_max: Ratio,

        // what position uncertainty is considered "acceptable". controller will
        // slowly accumulate uncertainty over time, depending on settings below.
        // once uncertainty exceeds the max value, a calibration will be
        // performed. uncertainty will also be automatically compensate during
        // full down/up movements. 5% is reasonable here
        pub position_uncertainty_max: Ratio,

        // constant + relative uncertainty coming from each movement (ax+b
        // formula). set this depending on how precise are your configuration.
        pub position_uncertainty_move_constant: Ratio,
        pub position_uncertainty_move_relative: Ratio,
    }
    impl Configuration {
        pub fn validate(&self) -> Result<(), Error> {
            ensure!(self.travel_down > Duration::ZERO);
            ensure!(self.travel_up > Duration::ZERO);

            ensure!(self.position_offset_max > Ratio::zero());

            ensure!(
                self.position_uncertainty_max > Ratio::zero()
                    || (self.position_uncertainty_move_constant == Ratio::zero()
                        && self.position_uncertainty_move_relative == Ratio::zero())
            );

            Ok(())
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
    pub struct Position {
        pub position: Ratio,    // 0.0 - fully down, 1.0 - fully up.
        pub uncertainty: Ratio, // position may be +- uncertainty
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
    pub enum Direction {
        Down,
        Up,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct StateCalibrating {
        // primary parameters
        pub started: Instant,
        pub direction: Direction,

        // derived calculations
        pub duration: Duration, // total, including start delay, dead time and travel time
    }
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct StateStopped {
        // primary parameters
        pub position: Position,
    }
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct StateMoving {
        // primary parameters
        pub started: Instant,
        pub started_position: Position,
        pub setpoint: Ratio,

        // derived calculations
        pub duration: Duration, // total, including start delay, dead time and travel time
        pub direction: Direction,
    }
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum State {
        // State in which we don't know where we are. This state is kept until
        // users selects any setpoint position. Once user selects any setpoint
        // position, we perform calibration in direction closer to the setpoint.
        Uncalibrated,
        // State in which we are calibrating (establishing the position, by
        // running fully down or up), to stop on limit switch.
        Calibrating(StateCalibrating),
        // We are stopped with known position.
        Stopped(StateStopped),
        // We are moving from known position.
        Moving(StateMoving),
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct Tick {
        pub output: Option<Direction>,
        pub next: Option<Duration>,
    }

    #[derive(Debug)]
    pub struct Controller {
        configuration: Configuration,
        state: State,
    }
    impl Controller {
        pub fn new(
            configuration: Configuration,
            position: Option<Position>,
        ) -> Self {
            configuration.validate().unwrap();

            let state = match position {
                Some(position) => State::Stopped(StateStopped { position }),
                None => State::Uncalibrated,
            };

            Self {
                configuration,
                state,
            }
        }

        pub fn configuration(&self) -> &Configuration {
            &self.configuration
        }
        pub fn state(&self) -> &State {
            &self.state
        }

        pub fn tick(
            &mut self,
            now: Instant,
            setpoint: Option<Ratio>,
        ) -> Tick {
            loop {
                // NOTE: since some state transitions may be transient, we return only from
                // terminal states

                match self.state {
                    State::Uncalibrated => {
                        // if user didn't select desired position - don't move
                        let setpoint = match setpoint {
                            Some(setpoint) => setpoint,
                            None => {
                                break Tick {
                                    output: None,
                                    next: None,
                                };
                            }
                        };

                        // user selected some position, so we can establish direction and duration
                        let direction = self.calibrating_direction_resolve(&setpoint);

                        let duration = self.calibrating_duration_calculate(&direction);

                        self.state = State::Calibrating(StateCalibrating {
                            started: now,
                            direction,
                            duration,
                        });
                        continue;
                    }
                    State::Calibrating(state) => {
                        // if user doesn't want to move anymore - stop
                        let setpoint = match setpoint {
                            Some(setpoint) => setpoint,
                            None => {
                                // if user doesn't select the position anymore - stop and assume
                                // uncalibrated
                                self.state = State::Uncalibrated;
                                continue;
                            }
                        };

                        // the only condition in which we restart the state is when user changes
                        // direction
                        let direction = self.calibrating_direction_resolve(&setpoint);
                        if direction != state.direction {
                            // user changed the direction, so restart the movement from the
                            // beginning
                            let duration = self.calibrating_duration_calculate(&direction);

                            self.state = State::Calibrating(StateCalibrating {
                                started: now,
                                direction,
                                duration,
                            });
                            continue;
                        }

                        // how long are we in this state
                        let elapsed = now - state.started;

                        // if enough time has passed - stop and assume calibrated
                        if elapsed >= state.duration {
                            // we have completed the movement, assume calibrated and go back to
                            // stopped state

                            // our position depends on direction we were calibrating to
                            let position = match state.direction {
                                Direction::Down => Ratio::zero(),
                                Direction::Up => Ratio::full(),
                            };

                            // we are calibrated, so uncertainty is zero
                            let position = Position {
                                position,
                                uncertainty: Ratio::zero(),
                            };

                            self.state = State::Stopped(StateStopped { position });
                            continue;
                        } else if elapsed >= self.configuration.start_delay {
                            // we are in the movement period, keep the motor running
                            let next = state.duration - elapsed;

                            break Tick {
                                output: Some(state.direction),
                                next: Some(next),
                            };
                        } else {
                            // we are in the start delay period, keep motor stopped and tick when
                            // its ready
                            let next = self.configuration.start_delay - elapsed;

                            break Tick {
                                output: None,
                                next: Some(next),
                            };
                        }
                    }
                    State::Stopped(state) => {
                        // if user didn't select desired position - stay here
                        let setpoint = match setpoint {
                            Some(setpoint) => setpoint,
                            None => {
                                // we stay stopped
                                break Tick {
                                    output: None,
                                    next: None,
                                };
                            }
                        };

                        // calculate how far are we from the setpoint
                        let position_offset =
                            self.position_offset_calculate(&state.position, &setpoint);

                        // if we are close enough - stay here
                        if position_offset <= self.configuration.position_offset_max {
                            break Tick {
                                output: None,
                                next: None,
                            };
                        }

                        // we need to move, so if we have accumulated enough uncertainty - start
                        // with calibration
                        if state.position.uncertainty > self.configuration.position_uncertainty_max
                        {
                            self.state = State::Uncalibrated;
                            continue;
                        }

                        // we are calibrated, so we can initialize the target movement
                        let (duration, direction) =
                            self.moving_duration_direction_calculate(&state.position, &setpoint);

                        self.state = State::Moving(StateMoving {
                            started: now,
                            started_position: state.position,
                            setpoint,
                            duration,
                            direction,
                        });
                        continue;
                    }
                    State::Moving(state) => {
                        // how long are we in this state?
                        let elapsed = now - state.started;

                        // if user doesn't want to move anymore, go back to stopped sate
                        let setpoint = match setpoint {
                            Some(setpoint) => setpoint,
                            None => {
                                let position = self.moving_stop_position_calculate(
                                    &state.started_position,
                                    &state.direction,
                                    &elapsed,
                                );

                                self.state = State::Stopped(StateStopped { position });
                                continue;
                            }
                        };

                        // if user has changed the setpoint, handle the
                        // situation here.
                        if setpoint != state.setpoint {
                            // calculate the new movement, assuming we started from our initial
                            // point there are three possibilities:
                            // - new setpoint is behind starting position (so also behind us)
                            //   (`direction` won't match). in this case we have to restart whole
                            //   movement, we go there by transitioning to stopped state (which will
                            //   transition back here).
                            // - new setpoint is between starting point and us (so we already passed
                            //   it) (`duration` is lower). in this case the next if-check will jump
                            //   to stopped state which will restart the movement.
                            // - new setpoint is still ahead of us (`duration` is higher). we will
                            //   simply continue our movement until reaching new `duration`.
                            let (duration, direction) = self.moving_duration_direction_calculate(
                                &state.started_position,
                                &setpoint,
                            );

                            if direction != state.direction {
                                let position = self.moving_stop_position_calculate(
                                    &state.started_position,
                                    &state.direction,
                                    &elapsed,
                                );

                                self.state = State::Stopped(StateStopped { position });
                                continue;
                            }

                            // continue the movement using initial parameters
                            self.state = State::Moving(StateMoving {
                                started: state.started,
                                started_position: state.started_position,
                                setpoint,
                                duration,
                                direction,
                            });
                            continue;
                        }

                        // we are good to go, continue with current state
                        if elapsed >= state.duration {
                            // if enough time has passed go back to stopped state
                            let position = self.moving_stop_position_calculate(
                                &state.started_position,
                                &state.direction,
                                &elapsed,
                            );

                            self.state = State::Stopped(StateStopped { position });
                            continue;
                        } else if elapsed >= self.configuration.start_delay {
                            // we are in the movement period, keep the motor running
                            let next = state.duration - elapsed;

                            break Tick {
                                output: Some(state.direction),
                                next: Some(next),
                            };
                        } else {
                            // we are in the start delay period, keep motor stopped and tick when
                            // its ready
                            let next = self.configuration.start_delay - elapsed;

                            break Tick {
                                output: None,
                                next: Some(next),
                            };
                        }
                    }
                }
            }
        }

        fn dead_travel(
            &self,
            direction: &Direction,
        ) -> (Duration, Duration) {
            match direction {
                Direction::Down => (
                    // if going down
                    self.configuration.dead_down,
                    self.configuration.travel_down,
                ),
                Direction::Up => (
                    // if going up
                    self.configuration.dead_up,
                    self.configuration.travel_up,
                ),
            }
        }
        fn position_offset_calculate(
            &self,
            position: &Position,
            setpoint: &Ratio,
        ) -> Ratio {
            let position_offset =
                Ratio::from_f64((setpoint.to_f64() - position.position.to_f64()).abs()).unwrap();

            position_offset
        }

        fn calibrating_direction_resolve(
            &self,
            setpoint: &Ratio,
        ) -> Direction {
            // for calibration - use closest direction
            let direction = if *setpoint <= Ratio::from_f64(0.5).unwrap() {
                Direction::Down
            } else {
                Direction::Up
            };

            direction
        }
        fn calibrating_duration_calculate(
            &self,
            direction: &Direction,
        ) -> Duration {
            // make a full movement (1.0) + compensate constant error + compensate full
            // (implicit 1.0 *) relative error
            let position_offset = Multiplier::from_f64(
                // full movement
                1.0 +
                // compensate constant error
                self.configuration.position_uncertainty_move_constant.to_f64() +
                // compensate error of full movement (implicit 1.0
                // multiplied by the ratio, as we are doing full
                // movement)
                self.configuration.position_uncertainty_move_relative.to_f64(),
            )
            .unwrap();

            // calculate overall duration
            let (dead, travel) = self.dead_travel(direction);
            let duration =
                self.configuration.start_delay + dead + travel.mul_f64(position_offset.to_f64());

            duration
        }

        fn moving_duration_direction_calculate(
            &self,
            position: &Position,
            setpoint: &Ratio,
        ) -> (Duration, Direction) {
            // calculate the direction to move
            let direction = if *setpoint <= position.position {
                Direction::Down
            } else {
                Direction::Up
            };

            // calculate offset to move
            let position_offset = self.position_offset_calculate(position, setpoint);

            // if user wants full down or up movement, we can add an overdrive to
            // compensate current error without even user noticing.

            let position_overdrive_offset = Multiplier::from_f64(
                // base movement
                position_offset.to_f64()
                    + if *setpoint == Ratio::full() || *setpoint == Ratio::zero() {
                        // compensate uncertainty
                        position.uncertainty.to_f64()
                        // compensate current movement error
                            + self
                                .configuration
                                .position_uncertainty_move_constant
                                .to_f64()
                            + self
                                .configuration
                                .position_uncertainty_move_relative
                                .to_f64()
                                * position_offset.to_f64()
                    } else {
                        0.0
                    },
            )
            .unwrap();

            // calculate movement time
            let (dead, travel) = self.dead_travel(&direction);
            let duration = self.configuration.start_delay
                + dead
                + travel.mul_f64(position_overdrive_offset.to_f64());

            (duration, direction)
        }
        fn moving_stop_position_calculate(
            &self,
            started_position: &Position,
            direction: &Direction,
            elapsed: &Duration,
        ) -> Position {
            let (dead, travel) = self.dead_travel(direction);

            // (position + overdrive) offset, aka. total movement
            let position_overdrive_offset = Multiplier::from_f64(
                elapsed
                    .saturating_sub(self.configuration.start_delay)
                    .saturating_sub(dead)
                    .div_duration_f64(travel),
            )
            .unwrap();

            // if we haven't moved a bit, we can assume to be in the initial position
            if position_overdrive_offset <= Multiplier::zero() {
                return *started_position;
            }

            // position + (position offset + overdrive offset), aka. position not clamped
            let position_overdriven = started_position.position.to_f64()
                + position_overdrive_offset.to_f64()
                    * match direction {
                        Direction::Down => -1.0,
                        Direction::Up => 1.0,
                    };

            // position after the move (overdrive removed), aka. position clamped
            let position = Ratio::from_f64(position_overdriven.clamp(0.0, 1.0)).unwrap();

            // position offset (overdrive removed), aka. movement not including overdrive
            let position_offset =
                Ratio::from_f64((position.to_f64() - started_position.position.to_f64()).abs())
                    .unwrap();

            // overdrive amount, aka. how much did we move on top of position_offset
            let overdrive_offset =
                Multiplier::from_f64((position_overdriven - position.to_f64()).abs()).unwrap();

            // calculate how our uncertainty changes. we add:
            // - constant from every movement
            // - relative to movement length
            // - negative overdrive (compensating the error)
            // NOTE: we don't include overdrive in relative uncertainty, otherwise we would
            // always stay with positive error.
            let uncertainty_offset = 0.0
                + self
                    .configuration
                    .position_uncertainty_move_constant
                    .to_f64()
                + self
                    .configuration
                    .position_uncertainty_move_relative
                    .to_f64()
                    * position_offset.to_f64()
                - overdrive_offset.to_f64();

            let uncertainty = Ratio::from_f64(
                (started_position.uncertainty.to_f64() + uncertainty_offset).clamp(0.0, 1.0),
            )
            .unwrap();

            let position = Position {
                position,
                uncertainty,
            };

            position
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{
            Configuration, Controller, Direction, Position, State, StateCalibrating, StateMoving,
            StateStopped, Tick,
        };
        use crate::datatypes::ratio::Ratio;
        use approx::assert_relative_eq;
        use std::{
            sync::LazyLock,
            time::{Duration, Instant},
        };

        static CONFIGURATION: LazyLock<Configuration> = LazyLock::new(|| Configuration {
            travel_down: Duration::from_secs(9),
            travel_up: Duration::from_secs(11),
            dead_down: Duration::from_secs(4),
            dead_up: Duration::from_secs(3),
            start_delay: Duration::from_secs(1),
            position_offset_max: Ratio::from_f64(0.01).unwrap(),
            position_uncertainty_max: Ratio::from_f64(0.05).unwrap(),
            position_uncertainty_move_constant: Ratio::from_f64(0.0025).unwrap(),
            position_uncertainty_move_relative: Ratio::from_f64(0.005).unwrap(),
        });

        static START: LazyLock<Instant> = LazyLock::new(Instant::now);

        fn tick_validate(
            controller: &mut Controller,
            input_elapsed: Duration,
            input_setpoint: Option<Ratio>,
            expected_tick: Tick,
            expected_state: State,
        ) {
            let tick = controller.tick(*START + input_elapsed, input_setpoint);

            let Tick { output, next } = tick;
            let Tick {
                output: expected_output,
                next: expected_next,
            } = expected_tick;

            match (output, expected_output) {
                (None, None) => {}
                (Some(direction), Some(direction_expected)) => {
                    assert_eq!(direction, direction_expected);
                }
                _ => panic!(
                    "output mismatch, got {:?}, expecting {:?}",
                    tick.output, expected_tick.output
                ),
            }

            match (next, expected_next) {
                (None, None) => {}
                (Some(tick_next), Some(expected_tick_next)) => {
                    assert_relative_eq!(tick_next.as_secs_f64(), expected_tick_next.as_secs_f64());
                }
                (next, expected_next) => panic!(
                    "tick next mismatch, got {:?}, expecting {:?}",
                    next, expected_next
                ),
            }

            match (*controller.state(), expected_state) {
                (State::Uncalibrated, State::Uncalibrated) => {}
                (
                    State::Calibrating(StateCalibrating {
                        started,
                        direction,
                        duration,
                    }),
                    State::Calibrating(StateCalibrating {
                        started: expected_started,
                        direction: expected_direction,
                        duration: expected_duration,
                    }),
                ) => {
                    assert_relative_eq!(
                        (started - *START).as_secs_f64(),
                        (expected_started - *START).as_secs_f64()
                    );
                    assert_eq!(direction, expected_direction);
                    assert_eq!(duration, expected_duration);
                }
                (
                    State::Stopped(StateStopped { position }),
                    State::Stopped(StateStopped {
                        position: expected_position,
                    }),
                ) => {
                    let Position {
                        position,
                        uncertainty,
                    } = position;
                    let Position {
                        position: expected_position,
                        uncertainty: expected_uncertainty,
                    } = expected_position;

                    assert_relative_eq!(position, expected_position);
                    assert_relative_eq!(uncertainty, expected_uncertainty);
                }
                (
                    State::Moving(StateMoving {
                        started,
                        started_position,
                        setpoint,
                        duration,
                        direction,
                    }),
                    State::Moving(StateMoving {
                        started: expected_started,
                        started_position: expected_started_position,
                        setpoint: expected_setpoint,
                        duration: expected_duration,
                        direction: expected_direction,
                    }),
                ) => {
                    assert_relative_eq!(
                        (started - *START).as_secs_f64(),
                        (expected_started - *START).as_secs_f64()
                    );

                    let Position {
                        position,
                        uncertainty,
                    } = started_position;
                    let Position {
                        position: expected_position,
                        uncertainty: expected_uncertainty,
                    } = expected_started_position;

                    assert_relative_eq!(position, expected_position);
                    assert_relative_eq!(uncertainty, expected_uncertainty);

                    assert_relative_eq!(setpoint, expected_setpoint);

                    assert_eq!(direction, expected_direction);

                    assert_relative_eq!(duration.as_secs_f64(), expected_duration.as_secs_f64());
                }
                (state, expected_state) => panic!(
                    "expected state mismatch, got: {:?}, expected:{:?}",
                    state, expected_state
                ),
            }
        }

        #[test]
        fn test_uncalibrated_to_calibrated() {
            let mut controller = Controller::new(*CONFIGURATION, None);

            // initial condition - should not move
            tick_validate(
                &mut controller,
                Duration::from_secs(0),
                None,
                Tick {
                    output: None,
                    next: None,
                },
                State::Uncalibrated,
            );

            // set a value of 1.0 -> should assume full movement from 0.0 to 1.0
            // time should be 1 s + stopped for start_delay
            tick_validate(
                &mut controller,
                Duration::from_secs(1),
                Some(Ratio::full()),
                Tick {
                    output: None,
                    next: Some(Duration::from_secs(1)),
                },
                State::Calibrating(StateCalibrating {
                    started: *START + Duration::from_secs(1),
                    direction: Direction::Up,
                    duration: Duration::from_secs_f64(1.0 + 3.0 + 11.0 * (1.0 + 0.0025 + 0.005)),
                }),
            );

            // set a value of 1.0 -> should assume full movement from 0.0 to 1.0
            // time should be 1 s + stopped for start_delay
            // time should be full movement + constant compensation + variable compensation
            tick_validate(
                &mut controller,
                Duration::from_secs(2),
                Some(Ratio::full()),
                Tick {
                    output: Some(Direction::Up),
                    next: Some(Duration::from_secs_f64(3.0 + 11.0 * (1.0 + 0.0025 + 0.005))),
                },
                State::Calibrating(StateCalibrating {
                    started: *START + Duration::from_secs(1),
                    direction: Direction::Up,
                    duration: Duration::from_secs_f64(1.0 + 3.0 + 11.0 * (1.0 + 0.0025 + 0.005)),
                }),
            );

            // almost finished, should have approx one second left
            // after this time it should be calibrated and stopped
            tick_validate(
                &mut controller,
                Duration::from_secs_f64(2.0 + 3.0 + 11.0 * (1.0 + 0.0025 + 0.005) - 1.0),
                Some(Ratio::full()),
                Tick {
                    output: Some(Direction::Up),
                    next: Some(Duration::from_secs(1)),
                },
                State::Calibrating(StateCalibrating {
                    started: *START + Duration::from_secs(1),
                    direction: Direction::Up,
                    duration: Duration::from_secs_f64(1.0 + 3.0 + 11.0 * (1.0 + 0.0025 + 0.005)),
                }),
            );

            // after this time it should be calibrated and stopped
            tick_validate(
                &mut controller,
                Duration::from_secs_f64(2.0 + 3.0 + 11.0 * (1.0 + 0.0025 + 0.005)),
                Some(Ratio::full()),
                Tick {
                    output: None,
                    next: None,
                },
                State::Stopped(StateStopped {
                    position: Position {
                        position: Ratio::full(),
                        uncertainty: Ratio::zero(),
                    },
                }),
            );
        }

        #[test]
        fn test_stopped_stays_stopped_for_small_error() {
            let mut controller = Controller::new(
                *CONFIGURATION,
                Some(Position {
                    position: Ratio::from_f64(0.5).unwrap(),
                    uncertainty: Ratio::zero(),
                }),
            );

            // no input provided, should stay in stopped state
            tick_validate(
                &mut controller,
                Duration::from_secs(0),
                None,
                Tick {
                    output: None,
                    next: None,
                },
                State::Stopped(StateStopped {
                    position: Position {
                        position: Ratio::from_f64(0.5).unwrap(),
                        uncertainty: Ratio::zero(),
                    },
                }),
            );

            // set setpoint to 0.509, shouldn't move
            tick_validate(
                &mut controller,
                Duration::from_secs(1),
                Some(Ratio::from_f64(0.509).unwrap()),
                Tick {
                    output: None,
                    next: None,
                },
                State::Stopped(StateStopped {
                    position: Position {
                        position: Ratio::from_f64(0.5).unwrap(),
                        uncertainty: Ratio::zero(),
                    },
                }),
            );

            // set setpoint to 0.491, shouldn't move
            tick_validate(
                &mut controller,
                Duration::from_secs(2),
                Some(Ratio::from_f64(0.491).unwrap()),
                Tick {
                    output: None,
                    next: None,
                },
                State::Stopped(StateStopped {
                    position: Position {
                        position: Ratio::from_f64(0.5).unwrap(),
                        uncertainty: Ratio::zero(),
                    },
                }),
            );

            // set setpoint to 0.511, should initialize movement, but still wait for start
            // delay
            tick_validate(
                &mut controller,
                Duration::from_secs(3),
                Some(Ratio::from_f64(0.511).unwrap()),
                Tick {
                    output: None,
                    next: Some(Duration::from_secs(1)),
                },
                State::Moving(StateMoving {
                    started: *START + Duration::from_secs(3),
                    started_position: Position {
                        position: Ratio::from_f64(0.50).unwrap(),
                        uncertainty: Ratio::zero(),
                    },
                    setpoint: Ratio::from_f64(0.511).unwrap(),
                    duration: Duration::from_secs_f64(1.0 + 3.0 + 11.0 * 0.011),
                    direction: Direction::Up,
                }),
            );

            // reset setpoint to be down, within 1 second of start delay move
            // down, we should have new move scheduled down
            tick_validate(
                &mut controller,
                Duration::from_secs(4),
                Some(Ratio::from_f64(0.489).unwrap()),
                Tick {
                    output: None,
                    next: Some(Duration::from_secs(1)),
                },
                State::Moving(StateMoving {
                    started: *START + Duration::from_secs(4),
                    started_position: Position {
                        position: Ratio::from_f64(0.50).unwrap(),
                        uncertainty: Ratio::zero(),
                    },
                    setpoint: Ratio::from_f64(0.489).unwrap(),
                    duration: Duration::from_secs_f64(1.0 + 4.0 + 9.0 * 0.011),
                    direction: Direction::Down,
                }),
            );

            // after start delay it should start moving
            tick_validate(
                &mut controller,
                Duration::from_secs(5),
                Some(Ratio::from_f64(0.489).unwrap()),
                Tick {
                    output: Some(Direction::Down),
                    next: Some(Duration::from_secs_f64(4.0 + 9.0 * 0.011)),
                },
                State::Moving(StateMoving {
                    started: *START + Duration::from_secs(4),
                    started_position: Position {
                        position: Ratio::from_f64(0.50).unwrap(),
                        uncertainty: Ratio::zero(),
                    },
                    setpoint: Ratio::from_f64(0.489).unwrap(),
                    duration: Duration::from_secs_f64(1.0 + 4.0 + 9.0 * 0.011),
                    direction: Direction::Down,
                }),
            );
        }

        #[test]
        fn test_stopped_calibrates_for_large_uncertainty() {
            let mut controller = Controller::new(
                *CONFIGURATION,
                Some(Position {
                    position: Ratio::from_f64(0.1).unwrap(),
                    uncertainty: Ratio::from_f64(0.0498).unwrap(),
                }),
            );

            // no input provided, should stay in stopped state
            tick_validate(
                &mut controller,
                Duration::from_secs(0),
                None,
                Tick {
                    output: None,
                    next: None,
                },
                State::Stopped(StateStopped {
                    position: Position {
                        position: Ratio::from_f64(0.1).unwrap(),
                        uncertainty: Ratio::from_f64(0.0498).unwrap(),
                    },
                }),
            );

            // allow some movement up (0.1 -> 0.9), should build up more uncertainty
            tick_validate(
                &mut controller,
                Duration::from_secs(1),
                Some(Ratio::from_f64(0.9).unwrap()),
                Tick {
                    output: None,
                    next: Some(Duration::from_secs(1)), // start delay
                },
                State::Moving(StateMoving {
                    started: *START + Duration::from_secs(1),
                    started_position: Position {
                        position: Ratio::from_f64(0.1).unwrap(),
                        uncertainty: Ratio::from_f64(0.0498).unwrap(),
                    },
                    setpoint: Ratio::from_f64(0.9).unwrap(),
                    duration: Duration::from_secs_f64(1.0 + 3.0 + 11.0 * 0.8),
                    direction: Direction::Up,
                }),
            );

            tick_validate(
                &mut controller,
                Duration::from_secs(2),
                Some(Ratio::from_f64(0.9).unwrap()),
                Tick {
                    output: Some(Direction::Up),
                    next: Some(Duration::from_secs_f64(3.0 + 11.0 * 0.8)), // move time
                },
                State::Moving(StateMoving {
                    started: *START + Duration::from_secs(1),
                    started_position: Position {
                        position: Ratio::from_f64(0.1).unwrap(),
                        uncertainty: Ratio::from_f64(0.0498).unwrap(),
                    },
                    setpoint: Ratio::from_f64(0.9).unwrap(),
                    duration: Duration::from_secs_f64(1.0 + 3.0 + 11.0 * 0.8),
                    direction: Direction::Up,
                }),
            );

            // after movement time we should be stopped and have enough uncertainty so that
            // next move triggers the recalibration
            tick_validate(
                &mut controller,
                Duration::from_secs_f64(2.0 + 3.0 + 11.0 * 0.8),
                Some(Ratio::from_f64(0.9).unwrap()),
                Tick {
                    output: None,
                    next: None,
                },
                State::Stopped(StateStopped {
                    position: Position {
                        position: Ratio::from_f64(0.9).unwrap(),
                        uncertainty: Ratio::from_f64(0.0498 + 0.0025 + 0.8 * 0.005).unwrap(),
                    },
                }),
            );

            // start the next move and check
            tick_validate(
                &mut controller,
                Duration::from_secs(15),
                Some(Ratio::from_f64(0.1).unwrap()),
                Tick {
                    output: None,
                    next: Some(Duration::from_secs(1)), // start delay
                },
                State::Calibrating(StateCalibrating {
                    started: *START + Duration::from_secs(15),
                    direction: Direction::Down,
                    duration: Duration::from_secs_f64(1.0 + 4.0 + 9.0 * (1.0 + 0.0025 + 0.005)),
                }),
            );
        }

        #[test]
        fn moving_respects_timing() {
            let mut controller = Controller::new(
                *CONFIGURATION,
                Some(Position {
                    position: Ratio::from_f64(0.9).unwrap(),
                    uncertainty: Ratio::zero(),
                }),
            );

            // provide movement to 0.3
            // initially should provide 1 second startup delay with no movement
            tick_validate(
                &mut controller,
                Duration::from_secs(0),
                Some(Ratio::from_f64(0.3).unwrap()),
                Tick {
                    output: None,
                    next: Some(Duration::from_secs(1)),
                },
                State::Moving(StateMoving {
                    started: *START,
                    started_position: Position {
                        position: Ratio::from_f64(0.9).unwrap(),
                        uncertainty: Ratio::zero(),
                    },
                    setpoint: Ratio::from_f64(0.3).unwrap(),
                    duration: Duration::from_secs_f64(1.0 + 4.0 + 9.0 * 0.6),
                    direction: Direction::Down,
                }),
            );

            // after this one second should start moving
            // it should take 4 seconds dead + 0.6 of movement * 9 seconds down
            tick_validate(
                &mut controller,
                Duration::from_secs(1),
                Some(Ratio::from_f64(0.3).unwrap()),
                Tick {
                    output: Some(Direction::Down),
                    next: Some(Duration::from_secs_f64(4.0 + 9.0 * 0.6)),
                },
                State::Moving(StateMoving {
                    started: *START,
                    started_position: Position {
                        position: Ratio::from_f64(0.9).unwrap(),
                        uncertainty: Ratio::zero(),
                    },
                    setpoint: Ratio::from_f64(0.3).unwrap(),
                    duration: Duration::from_secs_f64(1.0 + 4.0 + 9.0 * 0.6),
                    direction: Direction::Down,
                }),
            );

            // one second before this time we increase the setpoint to 0.2, should calculate
            // missing second of movement + additional 9.0 * 0.1
            tick_validate(
                &mut controller,
                Duration::from_secs_f64(1.0 + 4.0 + 9.0 * 0.6 - 1.0),
                Some(Ratio::from_f64(0.2).unwrap()),
                Tick {
                    output: Some(Direction::Down),
                    next: Some(Duration::from_secs_f64(1.0 + 9.0 * 0.1)),
                },
                State::Moving(StateMoving {
                    started: *START,
                    started_position: Position {
                        position: Ratio::from_f64(0.9).unwrap(),
                        uncertainty: Ratio::zero(),
                    },
                    setpoint: Ratio::from_f64(0.2).unwrap(),
                    duration: Duration::from_secs_f64(1.0 + 4.0 + 9.0 * 0.7),
                    direction: Direction::Down,
                }),
            );

            // and we overdue this by 0.005s, so it should end stopped slightly further, but
            // still acceptable
            tick_validate(
                &mut controller,
                Duration::from_secs_f64(1.0 + 4.0 + 9.0 * 0.7 + 0.005),
                Some(Ratio::from_f64(0.2).unwrap()),
                Tick {
                    output: None,
                    next: None,
                },
                State::Stopped(StateStopped {
                    position: Position {
                        position: Ratio::from_f64(0.2 - 1.0 / 9.0 * 0.005).unwrap(),
                        uncertainty: Ratio::from_f64(0.0025 + (0.7 + 1.0 / 9.0 * 0.005) * 0.005)
                            .unwrap(),
                    },
                }),
            );
        }

        #[test]
        fn moving_recalibrates_for_full_movement() {
            let mut controller = Controller::new(
                *CONFIGURATION,
                Some(Position {
                    position: Ratio::from_f64(0.1).unwrap(),
                    uncertainty: Ratio::from_f64(0.04).unwrap(),
                }),
            );

            // full (up) movement should provide compensation
            // starting with 1.0 delay
            tick_validate(
                &mut controller,
                Duration::from_secs(0),
                Some(Ratio::full()),
                Tick {
                    output: None,
                    next: Some(Duration::from_secs(1)),
                },
                State::Moving(StateMoving {
                    started: *START,
                    started_position: Position {
                        position: Ratio::from_f64(0.1).unwrap(),
                        uncertainty: Ratio::from_f64(0.04).unwrap(),
                    },
                    setpoint: Ratio::full(),
                    duration: Duration::from_secs_f64(
                        1.0 + 3.0 + 11.0 * (0.9 + 0.04 + 0.0025 + 0.9 * 0.005),
                    ),
                    direction: Direction::Up,
                }),
            );

            // now should provide time enough to compensate the error
            tick_validate(
                &mut controller,
                Duration::from_secs(1),
                Some(Ratio::full()),
                Tick {
                    output: Some(Direction::Up),
                    next: Some(Duration::from_secs_f64(
                        // 3.0 delay + 0.9 movement + 0.04 accumulated compensation + 0.0025
                        // current constant compensation + 0.9 * 0.005 current relative
                        // compensation
                        3.0 + 11.0 * (0.9 + 0.04 + 0.0025 + 0.9 * 0.005),
                    )),
                },
                State::Moving(StateMoving {
                    started: *START,
                    started_position: Position {
                        position: Ratio::from_f64(0.1).unwrap(),
                        uncertainty: Ratio::from_f64(0.04).unwrap(),
                    },
                    setpoint: Ratio::full(),
                    duration: Duration::from_secs_f64(
                        1.0 + 3.0 + 11.0 * (0.9 + 0.04 + 0.0025 + 0.9 * 0.005),
                    ),
                    direction: Direction::Up,
                }),
            );

            // once we reach there, our error should be zero
            tick_validate(
                &mut controller,
                Duration::from_secs_f64(1.0 + 3.0 + 11.0 * (0.9 + 0.04 + 0.0025 + 0.9 * 0.005)),
                Some(Ratio::full()),
                Tick {
                    output: None,
                    next: None,
                },
                State::Stopped(StateStopped {
                    position: Position {
                        position: Ratio::full(),
                        uncertainty: Ratio::zero(),
                    },
                }),
            );
        }

        #[test]
        fn test_moving_stopped_correct_position() {
            let mut controller = Controller::new(
                *CONFIGURATION,
                Some(Position {
                    position: Ratio::from_f64(0.1).unwrap(),
                    uncertainty: Ratio::zero(),
                }),
            );

            // begin movement up to 0.9
            tick_validate(
                &mut controller,
                Duration::from_secs(0),
                Some(Ratio::from_f64(0.9).unwrap()),
                Tick {
                    output: None,
                    next: Some(Duration::from_secs(1)), // start delay
                },
                State::Moving(StateMoving {
                    started: *START + Duration::from_secs(0),
                    started_position: Position {
                        position: Ratio::from_f64(0.1).unwrap(),
                        uncertainty: Ratio::zero(),
                    },
                    setpoint: Ratio::from_f64(0.9).unwrap(),
                    duration: Duration::from_secs_f64(1.0 + 3.0 + 11.0 * 0.8),
                    direction: Direction::Up,
                }),
            );

            // movement should take start delay (1s) + dead (3s) + movement (11.0 * 0.8)
            // start when it does 1/4 of the actual movement, so 1s + 3s + 11.0 * 1/4 * 0.8
            // = 6.2s
            tick_validate(
                &mut controller,
                Duration::from_secs_f64(6.2),
                None,
                Tick {
                    output: None,
                    next: None,
                },
                State::Stopped(StateStopped {
                    position: Position {
                        position: Ratio::from_f64(0.3).unwrap(),
                        uncertainty: Ratio::from_f64(0.0025 + 0.25 * 0.8 * 0.005).unwrap(), /* 0.0025 constant + 1/4 of 0.8 movement * 0.005 relative */
                    },
                }),
            );
        }

        #[test]
        fn test_moving_direction_inversion() {
            let mut controller = Controller::new(
                *CONFIGURATION,
                Some(Position {
                    position: Ratio::from_f64(0.1).unwrap(),
                    uncertainty: Ratio::zero(),
                }),
            );

            // setting up movement should begin normally (start delay)
            tick_validate(
                &mut controller,
                Duration::from_secs(0),
                Some(Ratio::full()),
                Tick {
                    output: None,
                    next: Some(Duration::from_secs(1)),
                },
                State::Moving(StateMoving {
                    started: *START,
                    started_position: Position {
                        position: Ratio::from_f64(0.1).unwrap(),
                        uncertainty: Ratio::zero(),
                    },
                    setpoint: Ratio::full(),
                    duration: Duration::from_secs_f64(
                        1.0 + 3.0 + 11.0 * (0.9 + 0.0025 + 0.005 * 0.9),
                    ),
                    direction: Direction::Up,
                }),
            );

            // after one second should begin to move
            tick_validate(
                &mut controller,
                Duration::from_secs(1),
                Some(Ratio::full()),
                Tick {
                    output: Some(Direction::Up),
                    next: Some(Duration::from_secs_f64(
                        3.0 + 11.0 * (0.9 + 0.0025 + 0.005 * 0.9),
                    )),
                },
                State::Moving(StateMoving {
                    started: *START,
                    started_position: Position {
                        position: Ratio::from_f64(0.1).unwrap(),
                        uncertainty: Ratio::zero(),
                    },
                    setpoint: Ratio::full(),
                    duration: Duration::from_secs_f64(
                        1.0 + 3.0 + 11.0 * (0.9 + 0.0025 + 0.005 * 0.9),
                    ),
                    direction: Direction::Up,
                }),
            );

            // we invert it in the 3/4 of overdrive, should have 3/4 compensated + be in
            // start delay again
            tick_validate(
                &mut controller,
                Duration::from_secs_f64(
                    1.0 + 3.0 + 11.0 * (0.9 + (0.0025 + 0.005 * 0.9) * 3.0 / 4.0),
                ),
                Some(Ratio::zero()),
                Tick {
                    output: None,
                    next: Some(Duration::from_secs(1)),
                },
                State::Moving(StateMoving {
                    started: *START
                        + Duration::from_secs_f64(
                            1.0 + 3.0 + 11.0 * (0.9 + (0.0025 + 0.005 * 0.9) * 3.0 / 4.0),
                        ),
                    started_position: Position {
                        position: Ratio::full(),
                        uncertainty: Ratio::from_f64(1.0 / 4.0 * (0.0025 + 0.005 * 0.9)).unwrap(),
                    },
                    setpoint: Ratio::zero(),
                    duration: Duration::from_secs_f64(
                        1.0 + 4.0
                            + 9.0
                                * (1.0
                                    + (1.0 / 4.0 * (0.0025 + 0.005 * 0.9))
                                    + 0.0025
                                    + 0.005 * 1.0),
                    ),
                    direction: Direction::Down,
                }),
            );
        }
    }
}
