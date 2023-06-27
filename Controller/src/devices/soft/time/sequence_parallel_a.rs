use crate::{
    datatypes::multiplier::Multiplier,
    devices,
    signals::{self, signal},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runtime::{Exited, Runnable},
    },
    web::{self, uri_cursor},
};
use anyhow::Context;
use async_trait::async_trait;
use core::panic;
use futures::{
    future::{BoxFuture, FutureExt},
    join,
    stream::StreamExt,
};
use itertools::{izip, zip_eq, Itertools};
use parking_lot::RwLock;
use serde::Serialize;
use std::{
    borrow::Cow,
    cmp::min,
    collections::HashMap,
    iter::{self, repeat_with},
    time::Duration,
};

#[derive(Debug)]
pub struct ConfigurationChannel {
    pub name: String,

    pub base_time: Duration,
    pub power_required: Multiplier,

    pub round_min: Duration,
    pub round_max: Duration,
}

#[derive(Debug)]
pub struct Configuration {
    pub power_max: Multiplier,
    pub channels: Box<[ConfigurationChannel]>,
}

#[derive(Clone, Copy, Debug)]
enum StateDeviceDisabledChannel {
    Disabled,
    Paused,
    Enabled,
}

#[derive(Clone, Copy, Debug)]
enum StateDevicePausedChannel {
    Disabled,
    Paused { queue: Duration },
    Enabled { queue: Duration },
}

#[derive(Clone, Copy, Debug)]
enum StateDeviceEnabledChannel {
    Disabled,
    Paused {
        queue: Duration,
    },
    EnabledQueued {
        queue: Duration,
        order_index: i64,
    },
    // This is the only state in which output is enabled and power is used
    EnabledActive {
        queue: Duration, // not including round
        round: Duration,
    },
}

#[derive(Debug)]
enum StateDevice {
    Disabled {
        channels: Box<[StateDeviceDisabledChannel]>,
    },
    Paused {
        channels: Box<[StateDevicePausedChannel]>,
    },
    Enabled {
        channels: Box<[StateDeviceEnabledChannel]>,
        order_index_last: u64,
    },
}

#[derive(Debug)]
struct State {
    device: StateDevice,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,
    state: RwLock<State>,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_add_all: signal::event_target_queued::Signal<Multiplier>,
    signal_power: signal::state_source::Signal<Multiplier>,
    signal_outputs: Box<[signal::state_source::Signal<bool>]>,

    gui_summary_waker: devices::gui_summary::Waker,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        // TODO: precondition check
        // every channel must have power_required <= device power_max
        // base_time > 0
        // round_min > 0
        // round_max > 0

        let channels_count = configuration.channels.len();

        let state_device = StateDevice::Enabled {
            channels: iter::repeat(StateDeviceEnabledChannel::EnabledQueued {
                queue: Duration::ZERO,
                order_index: 0,
            })
            .take(channels_count)
            .collect(),
            order_index_last: 0,
        };
        let state = State {
            device: state_device,
        };

        Self {
            configuration,
            state: RwLock::new(state),

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_add_all: signal::event_target_queued::Signal::<Multiplier>::new(),
            signal_power: signal::state_source::Signal::<Multiplier>::new(Some(Multiplier::zero())),
            signal_outputs: repeat_with(|| signal::state_source::Signal::<bool>::new(Some(false)))
                .take(channels_count)
                .collect(),

            gui_summary_waker: devices::gui_summary::Waker::new(),
        }
    }

    pub fn device_disable(&self) {
        let mut state = self.state.write();

        let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device {
            StateDevice::Disabled { .. } => {}
            StateDevice::Paused { ref channels } => {
                let channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        StateDevicePausedChannel::Disabled => StateDeviceDisabledChannel::Disabled,
                        StateDevicePausedChannel::Paused { .. } => {
                            StateDeviceDisabledChannel::Paused
                        }
                        StateDevicePausedChannel::Enabled { .. } => {
                            StateDeviceDisabledChannel::Enabled
                        }
                    })
                    .collect();

                state.device = StateDevice::Disabled { channels };

                gui_summary_changed = true;
            }
            StateDevice::Enabled { ref channels, .. } => {
                let channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        StateDeviceEnabledChannel::Disabled => StateDeviceDisabledChannel::Disabled,
                        StateDeviceEnabledChannel::Paused { .. } => {
                            StateDeviceDisabledChannel::Paused
                        }
                        StateDeviceEnabledChannel::EnabledQueued { .. } => {
                            StateDeviceDisabledChannel::Enabled
                        }
                        StateDeviceEnabledChannel::EnabledActive { .. } => {
                            // outputs and power will be zeroed at the end
                            StateDeviceDisabledChannel::Enabled
                        }
                    })
                    .collect();

                state.device = StateDevice::Disabled { channels };

                gui_summary_changed = true;
            }
        }

        // disable all channels
        if self.signal_power.set_one(Some(Multiplier::zero())) {
            signal_sources_changed = true;
        }
        for signal_output in self.signal_outputs.iter() {
            if signal_output.set_one(Some(false)) {
                signal_sources_changed = true;
            }
        }

        if signal_sources_changed {
            self.signals_sources_changed_waker.wake();
        }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }
    pub fn device_pause(&self) {
        let mut state = self.state.write();

        let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device {
            StateDevice::Disabled { ref channels } => {
                let channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        StateDeviceDisabledChannel::Disabled => StateDevicePausedChannel::Disabled,
                        StateDeviceDisabledChannel::Paused => StateDevicePausedChannel::Paused {
                            queue: Duration::ZERO,
                        },
                        StateDeviceDisabledChannel::Enabled => StateDevicePausedChannel::Enabled {
                            queue: Duration::ZERO,
                        },
                    })
                    .collect();

                state.device = StateDevice::Paused { channels };

                gui_summary_changed = true;
            }
            StateDevice::Paused { .. } => {}
            StateDevice::Enabled { ref channels, .. } => {
                let channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        StateDeviceEnabledChannel::Disabled => StateDevicePausedChannel::Disabled,
                        StateDeviceEnabledChannel::Paused { queue } => {
                            StateDevicePausedChannel::Paused { queue: *queue }
                        }
                        StateDeviceEnabledChannel::EnabledQueued { queue, .. } => {
                            StateDevicePausedChannel::Enabled { queue: *queue }
                        }
                        StateDeviceEnabledChannel::EnabledActive { queue, round } => {
                            // outputs and power will be zeroed at the end
                            StateDevicePausedChannel::Enabled {
                                queue: *queue + *round,
                            }
                        }
                    })
                    .collect();

                state.device = StateDevice::Paused { channels };

                gui_summary_changed = true;
            }
        }

        // disable all channels
        if self.signal_power.set_one(Some(Multiplier::zero())) {
            signal_sources_changed = true;
        }
        for signal_output in self.signal_outputs.iter() {
            if signal_output.set_one(Some(false)) {
                signal_sources_changed = true;
            }
        }

        if signal_sources_changed {
            self.signals_sources_changed_waker.wake();
        }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }
    pub fn device_enable(&self) {
        let mut state = self.state.write();

        // let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device {
            StateDevice::Disabled { ref channels } => {
                let channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        StateDeviceDisabledChannel::Disabled => StateDeviceEnabledChannel::Disabled,
                        StateDeviceDisabledChannel::Paused => StateDeviceEnabledChannel::Paused {
                            queue: Duration::ZERO,
                        },
                        StateDeviceDisabledChannel::Enabled => {
                            StateDeviceEnabledChannel::EnabledQueued {
                                queue: Duration::ZERO,
                                order_index: 0,
                            }
                        }
                    })
                    .collect();
                state.device = StateDevice::Enabled {
                    channels,
                    order_index_last: 0,
                };

                gui_summary_changed = true;
            }
            StateDevice::Paused { ref channels } => {
                let channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        StateDevicePausedChannel::Disabled => StateDeviceEnabledChannel::Disabled,
                        StateDevicePausedChannel::Paused { queue } => {
                            StateDeviceEnabledChannel::Paused { queue: *queue }
                        }
                        StateDevicePausedChannel::Enabled { queue } => {
                            StateDeviceEnabledChannel::EnabledQueued {
                                queue: *queue,
                                order_index: 0,
                            }
                        }
                    })
                    .collect();
                state.device = StateDevice::Enabled {
                    channels,
                    order_index_last: 0,
                };

                gui_summary_changed = true;
            }
            StateDevice::Enabled { .. } => {}
        }

        // if signal_sources_changed {
        //     self.signals_sources_changed_waker.wake();
        // }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }

    pub fn channel_disable(
        &self,
        channel_id: usize,
    ) {
        let mut state = self.state.write();

        let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device {
            StateDevice::Disabled { ref mut channels } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDeviceDisabledChannel::Disabled => {}
                    StateDeviceDisabledChannel::Paused | StateDeviceDisabledChannel::Enabled => {
                        *channel_state = StateDeviceDisabledChannel::Disabled;

                        gui_summary_changed = true;
                    }
                }
            }
            StateDevice::Paused { ref mut channels } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDevicePausedChannel::Disabled => {}
                    StateDevicePausedChannel::Paused { .. }
                    | StateDevicePausedChannel::Enabled { .. } => {
                        *channel_state = StateDevicePausedChannel::Disabled;

                        gui_summary_changed = true;
                    }
                }
            }
            StateDevice::Enabled {
                ref mut channels, ..
            } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDeviceEnabledChannel::Disabled => {}
                    StateDeviceEnabledChannel::Paused { .. }
                    | StateDeviceEnabledChannel::EnabledQueued { .. } => {
                        *channel_state = StateDeviceEnabledChannel::Disabled;

                        gui_summary_changed = true;
                    }
                    StateDeviceEnabledChannel::EnabledActive { .. } => {
                        *channel_state = StateDeviceEnabledChannel::Disabled;

                        if self.signal_outputs[channel_id].set_one(Some(false)) {
                            signal_sources_changed = true;
                        }
                        if self
                            .signal_power
                            .set_one(Some(self.power_calculate(channels)))
                        {
                            signal_sources_changed = true;
                        }

                        gui_summary_changed = true;
                    }
                }
            }
        }

        if signal_sources_changed {
            self.signals_sources_changed_waker.wake();
        }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }
    pub fn channel_pause(
        &self,
        channel_id: usize,
    ) {
        let mut state = self.state.write();

        let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device {
            StateDevice::Disabled { ref mut channels } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDeviceDisabledChannel::Disabled | StateDeviceDisabledChannel::Enabled => {
                        *channel_state = StateDeviceDisabledChannel::Paused;
                        gui_summary_changed = true;
                    }
                    StateDeviceDisabledChannel::Paused => {}
                }
            }
            StateDevice::Paused { ref mut channels } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDevicePausedChannel::Disabled => {
                        *channel_state = StateDevicePausedChannel::Paused {
                            queue: Duration::ZERO,
                        };
                        gui_summary_changed = true;
                    }
                    StateDevicePausedChannel::Enabled { queue } => {
                        *channel_state = StateDevicePausedChannel::Paused { queue: *queue };
                        gui_summary_changed = true;
                    }
                    StateDevicePausedChannel::Paused { .. } => {}
                }
            }
            StateDevice::Enabled {
                ref mut channels, ..
            } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDeviceEnabledChannel::Disabled => {
                        *channel_state = StateDeviceEnabledChannel::Paused {
                            queue: Duration::ZERO,
                        };
                        gui_summary_changed = true;
                    }
                    StateDeviceEnabledChannel::Paused { .. } => {}
                    StateDeviceEnabledChannel::EnabledQueued { queue, .. } => {
                        *channel_state = StateDeviceEnabledChannel::Paused { queue: *queue };
                        gui_summary_changed = true;
                    }
                    StateDeviceEnabledChannel::EnabledActive { queue, round, .. } => {
                        *channel_state = StateDeviceEnabledChannel::Paused {
                            queue: *queue + *round,
                        };

                        if self.signal_outputs[channel_id].set_one(Some(false)) {
                            signal_sources_changed = true;
                        }
                        if self
                            .signal_power
                            .set_one(Some(self.power_calculate(channels)))
                        {
                            signal_sources_changed = true;
                        }

                        gui_summary_changed = true;
                    }
                }
            }
        }

        if signal_sources_changed {
            self.signals_sources_changed_waker.wake();
        }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }
    pub fn channel_enable(
        &self,
        channel_id: usize,
    ) {
        let mut state = self.state.write();

        // let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device {
            StateDevice::Disabled { ref mut channels } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDeviceDisabledChannel::Disabled | StateDeviceDisabledChannel::Paused => {
                        *channel_state = StateDeviceDisabledChannel::Enabled;
                        gui_summary_changed = true;
                    }
                    StateDeviceDisabledChannel::Enabled => {}
                }
            }
            StateDevice::Paused { ref mut channels } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDevicePausedChannel::Disabled => {
                        *channel_state = StateDevicePausedChannel::Enabled {
                            queue: Duration::ZERO,
                        };
                        gui_summary_changed = true;
                    }
                    StateDevicePausedChannel::Paused { queue } => {
                        *channel_state = StateDevicePausedChannel::Enabled { queue: *queue };
                        gui_summary_changed = true;
                    }
                    StateDevicePausedChannel::Enabled { .. } => {}
                }
            }
            StateDevice::Enabled {
                ref mut channels,
                ref mut order_index_last,
            } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDeviceEnabledChannel::Disabled => {
                        *order_index_last += 1;

                        *channel_state = StateDeviceEnabledChannel::EnabledQueued {
                            queue: Duration::ZERO,
                            order_index: *order_index_last as i64,
                        };
                        gui_summary_changed = true;
                    }
                    StateDeviceEnabledChannel::Paused { queue } => {
                        *order_index_last += 1;

                        *channel_state = StateDeviceEnabledChannel::EnabledQueued {
                            queue: *queue,
                            order_index: *order_index_last as i64,
                        };
                        gui_summary_changed = true;
                    }
                    StateDeviceEnabledChannel::EnabledQueued { .. }
                    | StateDeviceEnabledChannel::EnabledActive { .. } => {}
                }
            }
        }

        // if signal_sources_changed {
        //     self.signals_sources_changed_waker.wake();
        // }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }
    pub fn channel_clear(
        &self,
        channel_id: usize,
    ) {
        let mut state = self.state.write();

        // let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device {
            StateDevice::Disabled { .. } => {}
            StateDevice::Paused { ref mut channels } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDevicePausedChannel::Disabled => {}
                    StateDevicePausedChannel::Paused { ref mut queue }
                    | StateDevicePausedChannel::Enabled { ref mut queue } => {
                        *queue = Duration::ZERO;
                        gui_summary_changed = true;
                    }
                }
            }
            StateDevice::Enabled {
                ref mut channels, ..
            } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDeviceEnabledChannel::Disabled => {}
                    StateDeviceEnabledChannel::Paused { ref mut queue }
                    | StateDeviceEnabledChannel::EnabledQueued { ref mut queue, .. }
                    | StateDeviceEnabledChannel::EnabledActive { ref mut queue, .. } => {
                        *queue = Duration::ZERO;
                        gui_summary_changed = true;
                    }
                }
            }
        }

        // if signal_sources_changed {
        //     self.signals_sources_changed_waker.wake();
        // }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }
    pub fn channel_add(
        &self,
        channel_id: usize,
        multiplier: Multiplier,
    ) {
        let mut state = self.state.write();

        // let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device {
            StateDevice::Disabled { .. } => {}
            StateDevice::Paused { ref mut channels } => {
                let channel_configuration = &self.configuration.channels[channel_id];
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDevicePausedChannel::Disabled => {}
                    StateDevicePausedChannel::Paused { ref mut queue }
                    | StateDevicePausedChannel::Enabled { ref mut queue } => {
                        *queue += channel_configuration.base_time.mul_f64(multiplier.to_f64());
                        gui_summary_changed = true;
                    }
                }
            }
            StateDevice::Enabled {
                ref mut channels, ..
            } => {
                let channel_configuration = &self.configuration.channels[channel_id];
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDeviceEnabledChannel::Disabled => {}
                    StateDeviceEnabledChannel::Paused { ref mut queue }
                    | StateDeviceEnabledChannel::EnabledQueued { ref mut queue, .. }
                    | StateDeviceEnabledChannel::EnabledActive { ref mut queue, .. } => {
                        *queue += channel_configuration.base_time.mul_f64(multiplier.to_f64());
                        gui_summary_changed = true;
                    }
                }
            }
        }

        // if signal_sources_changed {
        //     self.signals_sources_changed_waker.wake();
        // }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }
    pub fn channel_move_front(
        &self,
        channel_id: usize,
    ) {
        let mut state = self.state.write();

        // let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device {
            StateDevice::Disabled { .. } => {}
            StateDevice::Paused { .. } => {}
            StateDevice::Enabled {
                ref mut channels,
                ref mut order_index_last,
            } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDeviceEnabledChannel::Disabled
                    | StateDeviceEnabledChannel::Paused { .. }
                    | StateDeviceEnabledChannel::EnabledActive { .. } => {}
                    StateDeviceEnabledChannel::EnabledQueued {
                        ref mut order_index,
                        ..
                    } => {
                        *order_index_last += 1;
                        *order_index = -(*order_index_last as i64);

                        gui_summary_changed = true;
                    }
                }
            }
        }

        // if signal_sources_changed {
        //     self.signals_sources_changed_waker.wake();
        // }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }
    pub fn channel_move_back(
        &self,
        channel_id: usize,
    ) {
        let mut state = self.state.write();

        let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device {
            StateDevice::Disabled { .. } => {}
            StateDevice::Paused { .. } => {}
            StateDevice::Enabled {
                ref mut channels,
                ref mut order_index_last,
            } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    StateDeviceEnabledChannel::Disabled
                    | StateDeviceEnabledChannel::Paused { .. } => {}
                    StateDeviceEnabledChannel::EnabledQueued {
                        ref mut order_index,
                        ..
                    } => {
                        *order_index_last += 1;
                        *order_index = *order_index_last as i64;

                        gui_summary_changed = true;
                    }
                    StateDeviceEnabledChannel::EnabledActive { queue, round } => {
                        *order_index_last += 1;

                        *channel_state = StateDeviceEnabledChannel::EnabledQueued {
                            order_index: *order_index_last as i64,
                            queue: *queue + *round,
                        };

                        if self.signal_outputs[channel_id].set_one(Some(false)) {
                            signal_sources_changed = true;
                        }
                        if self
                            .signal_power
                            .set_one(Some(self.power_calculate(channels)))
                        {
                            signal_sources_changed = true;
                        }

                        gui_summary_changed = true;
                    }
                }
            }
        }

        if signal_sources_changed {
            self.signals_sources_changed_waker.wake();
        }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }

    pub fn channels_clear(&self) {
        let mut state = self.state.write();

        let mut gui_summary_changed = false;

        match state.device {
            StateDevice::Disabled { .. } => {}
            StateDevice::Paused {
                ref mut channels, ..
            } => {
                for channel_state in channels.iter_mut() {
                    match channel_state {
                        StateDevicePausedChannel::Disabled => {}
                        StateDevicePausedChannel::Paused { ref mut queue }
                        | StateDevicePausedChannel::Enabled { ref mut queue, .. } => {
                            *queue = Duration::ZERO;
                            gui_summary_changed = true;
                        }
                    }
                }
            }
            StateDevice::Enabled {
                ref mut channels, ..
            } => {
                for channel_state in channels.iter_mut() {
                    match channel_state {
                        StateDeviceEnabledChannel::Disabled => {}
                        StateDeviceEnabledChannel::Paused { ref mut queue, .. }
                        | StateDeviceEnabledChannel::EnabledQueued { ref mut queue, .. }
                        | StateDeviceEnabledChannel::EnabledActive { ref mut queue, .. } => {
                            *queue = Duration::ZERO;
                            gui_summary_changed = true;
                        }
                    }
                }
            }
        }

        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }
    pub fn channels_add(
        &self,
        multiplier: Multiplier,
    ) {
        let mut state = self.state.write();

        let mut gui_summary_changed = false;

        match state.device {
            StateDevice::Disabled { .. } => {}
            StateDevice::Paused {
                ref mut channels, ..
            } => {
                for (channel_configuration, channel_state) in
                    zip_eq(self.configuration.channels.iter(), channels.iter_mut())
                {
                    match channel_state {
                        StateDevicePausedChannel::Disabled => {}
                        StateDevicePausedChannel::Paused { ref mut queue }
                        | StateDevicePausedChannel::Enabled { ref mut queue, .. } => {
                            *queue += channel_configuration.base_time.mul_f64(multiplier.to_f64());
                            gui_summary_changed = true;
                        }
                    }
                }
            }
            StateDevice::Enabled {
                ref mut channels, ..
            } => {
                for (channel_configuration, channel_state) in
                    zip_eq(self.configuration.channels.iter(), channels.iter_mut())
                {
                    match channel_state {
                        StateDeviceEnabledChannel::Disabled => {}
                        StateDeviceEnabledChannel::Paused { ref mut queue, .. }
                        | StateDeviceEnabledChannel::EnabledQueued { ref mut queue, .. }
                        | StateDeviceEnabledChannel::EnabledActive { ref mut queue, .. } => {
                            *queue += channel_configuration.base_time.mul_f64(multiplier.to_f64());
                            gui_summary_changed = true;
                        }
                    }
                }
            }
        }

        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }

    const CHANNELS_TICK_INTERVAL: Duration = Duration::from_secs(1);
    fn channels_tick(&self) {
        let mut state = self.state.write();

        // we do ticks only when device is in running state
        let (channels, order_index_last) = match state.device {
            StateDevice::Enabled {
                ref mut channels,
                ref mut order_index_last,
            } => (channels, order_index_last),
            _ => return,
        };

        let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        let mut power_left = self.configuration.power_max;

        // in first iteration we remove channels that went to the end of their time
        for (channel_configuration, channel_state, signal_output) in izip!(
            self.configuration.channels.iter(),
            channels.iter_mut(),
            self.signal_outputs.iter()
        ) {
            match channel_state {
                StateDeviceEnabledChannel::Disabled
                | StateDeviceEnabledChannel::Paused { .. }
                | StateDeviceEnabledChannel::EnabledQueued { .. } => {}
                StateDeviceEnabledChannel::EnabledActive {
                    queue,
                    ref mut round,
                    ..
                } => {
                    *round = round.saturating_sub(Self::CHANNELS_TICK_INTERVAL);

                    if !round.is_zero() {
                        // channel can still run
                        power_left -= channel_configuration.power_required;
                    } else {
                        // channel time has ended, move it to the end of the queue
                        *order_index_last += 1;

                        *channel_state = StateDeviceEnabledChannel::EnabledQueued {
                            queue: *queue,
                            order_index: *order_index_last as i64,
                        };

                        if signal_output.set_one(Some(false)) {
                            signal_sources_changed = true;
                        }
                    }

                    gui_summary_changed = true;
                }
            }
        }

        // in the second iteration we add new channels if they are ready to be run
        // we process and try to enable them processed by order, until first failure, to prevent starvation
        let channel_ids = zip_eq(self.configuration.channels.iter(), channels.iter())
            .enumerate()
            .filter_map(
                |(channel_id, (channel_configuration, channel_state))| match channel_state {
                    // precondition: channel is in queued state and has enough time to start
                    StateDeviceEnabledChannel::EnabledQueued { queue, order_index } => {
                        if *queue >= channel_configuration.round_min {
                            Some((channel_id, order_index))
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
            )
            .sorted_by_key(|(_channel_id, order_index)| *order_index)
            .map(|(channel_id, _order_index)| channel_id)
            .collect::<Box<[_]>>();

        for channel_id in channel_ids.iter() {
            let channel_id = *channel_id;

            let channel_configuration = &self.configuration.channels[channel_id];
            let channel_state = &mut channels[channel_id];
            let signal_output = &self.signal_outputs[channel_id];

            match channel_state {
                // channel_ids should contain EnabledQueued only
                StateDeviceEnabledChannel::Disabled
                | StateDeviceEnabledChannel::Paused { .. }
                | StateDeviceEnabledChannel::EnabledActive { .. } => panic!(),

                StateDeviceEnabledChannel::EnabledQueued { queue, .. } => {
                    // total >= channel_configuration.round_min
                    // this precondition was checked during index preparing

                    if power_left >= channel_configuration.power_required {
                        let round = min(*queue, channel_configuration.round_max);
                        let queue = *queue - round;

                        *channel_state = StateDeviceEnabledChannel::EnabledActive { queue, round };

                        // enough power and time to start!
                        power_left -= channel_configuration.power_required;

                        if signal_output.set_one(Some(true)) {
                            signal_sources_changed = true;
                        }
                        gui_summary_changed = true;
                    } else {
                        // to prevent starvation we end iterating when first channel does not meet power condition
                        // this makes sure that iteration will stop here until this channel is ready to run
                        break;
                    }
                }
            }
        }

        let power = self.configuration.power_max - power_left;
        if self.signal_power.set_one(Some(power)) {
            signal_sources_changed = true;
        }

        if signal_sources_changed {
            self.signals_sources_changed_waker.wake();
        }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }

    fn power_calculate(
        &self,
        channels: &[StateDeviceEnabledChannel],
    ) -> Multiplier {
        zip_eq(self.configuration.channels.iter(), channels.iter())
            .map(|(configuration, state)| match state {
                StateDeviceEnabledChannel::EnabledActive { .. } => configuration.power_required,
                _ => Multiplier::zero(),
            })
            .sum::<Multiplier>()
    }

    fn signals_targets_changed(&self) {
        let value = self
            .signal_add_all
            .take_pending()
            .into_vec()
            .into_iter()
            .sum::<Multiplier>();

        if value > Multiplier::zero() {
            self.channels_add(value);
        }
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
        let tick_runner = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
            Self::CHANNELS_TICK_INTERVAL,
        ))
        .stream_take_until_exhausted(exit_flag.clone())
        .for_each(async move |_| {
            self.channels_tick();
        })
        .boxed();

        // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
        let signals_targets_changed_runner = self
            .signals_targets_changed_waker
            .stream()
            .stream_take_until_exhausted(exit_flag.clone())
            .for_each(async move |()| {
                self.signals_targets_changed();
            })
            .boxed();

        let _: ((), ()) = join!(tick_runner, signals_targets_changed_runner);

        Exited
    }
}

impl devices::Device for Device {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("soft/time/sequence_parallel_a")
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
    AddAll,
    Power,
    Output(usize),
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
        iter::empty()
            .chain([
                (
                    SignalIdentifier::AddAll,
                    &self.signal_add_all as &dyn signal::Base,
                ),
                (
                    SignalIdentifier::Power,
                    &self.signal_power as &dyn signal::Base,
                ),
            ])
            .chain(
                self.signal_outputs
                    .iter()
                    .enumerate()
                    .map(|(output_index, output_signal)| {
                        (
                            SignalIdentifier::Output(output_index),
                            output_signal as &dyn signal::Base,
                        )
                    }),
            )
            .collect()
    }
}

// TODO: use newtype inestead of f64
#[derive(Debug, Serialize)]
struct GuiSummaryConfigurationChannel {
    name: String,

    base_time_seconds: f64,
    power_required: f64,

    round_min_seconds: f64,
    round_max_seconds: f64,
}

#[derive(Debug, Serialize)]
struct GuiSummaryConfiguration {
    channels: Box<[GuiSummaryConfigurationChannel]>,
    power_max: f64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "state")]
enum GuiSummaryStateDisabledChannelState {
    Disabled,
    Paused,
    Enabled,
}
#[derive(Debug, Serialize)]
#[serde(tag = "state")]
enum GuiSummaryStatePausedChannelState {
    Disabled,
    Paused { queue_seconds: f64 },
    Enabled { queue_seconds: f64 },
}
#[derive(Debug, Serialize)]
#[serde(tag = "state")]
enum GuiSummaryStateEnabledChannelState {
    Disabled,
    Paused {
        queue_seconds: f64,
    },
    EnabledQueued {
        queue_seconds: f64,
        queue_position: Option<usize>,
    },
    EnabledActive {
        queue_seconds: f64,
        round_seconds: f64,
    },
}
#[derive(Debug, Serialize)]
#[serde(tag = "state")]
enum GuiSummaryState {
    Disabled {
        channels: Box<[GuiSummaryStateDisabledChannelState]>,
    },
    Paused {
        channels: Box<[GuiSummaryStatePausedChannelState]>,
    },
    Enabled {
        channels: Box<[GuiSummaryStateEnabledChannelState]>,
        power: f64,
    },
}
#[derive(Debug, Serialize)]
pub struct GuiSummary {
    configuration: GuiSummaryConfiguration,
    state: GuiSummaryState,
}
impl devices::gui_summary::Device for Device {
    fn waker(&self) -> &devices::gui_summary::Waker {
        &self.gui_summary_waker
    }

    type Value = GuiSummary;
    fn value(&self) -> Self::Value {
        let state = self.state.read();

        let gui_summary_configuration_channels = self
            .configuration
            .channels
            .iter()
            .map(|channel_configuration| GuiSummaryConfigurationChannel {
                name: channel_configuration.name.clone(),
                base_time_seconds: channel_configuration.base_time.as_secs_f64(),
                power_required: channel_configuration.power_required.to_f64(),
                round_min_seconds: channel_configuration.round_min.as_secs_f64(),
                round_max_seconds: channel_configuration.round_max.as_secs_f64(),
            })
            .collect();

        let gui_summary_configuration = GuiSummaryConfiguration {
            channels: gui_summary_configuration_channels,
            power_max: self.configuration.power_max.to_f64(),
        };

        let gui_summary_state = match state.device {
            StateDevice::Disabled { ref channels } => {
                let gui_channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        StateDeviceDisabledChannel::Disabled => {
                            GuiSummaryStateDisabledChannelState::Disabled
                        }
                        StateDeviceDisabledChannel::Paused => {
                            GuiSummaryStateDisabledChannelState::Paused
                        }
                        StateDeviceDisabledChannel::Enabled => {
                            GuiSummaryStateDisabledChannelState::Enabled
                        }
                    })
                    .collect();

                GuiSummaryState::Disabled {
                    channels: gui_channels,
                }
            }
            StateDevice::Paused { ref channels, .. } => {
                let gui_channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        StateDevicePausedChannel::Disabled => {
                            GuiSummaryStatePausedChannelState::Disabled
                        }
                        StateDevicePausedChannel::Paused { queue } => {
                            GuiSummaryStatePausedChannelState::Paused {
                                queue_seconds: queue.as_secs_f64(),
                            }
                        }
                        StateDevicePausedChannel::Enabled { queue } => {
                            GuiSummaryStatePausedChannelState::Enabled {
                                queue_seconds: queue.as_secs_f64(),
                            }
                        }
                    })
                    .collect();

                GuiSummaryState::Paused {
                    channels: gui_channels,
                }
            }
            StateDevice::Enabled { ref channels, .. } => {
                // channel_id -> 0-based queue position (ascending)
                let queued_positions = zip_eq(self.configuration.channels.iter(), channels.iter())
                    .enumerate()
                    .filter_map(|(channel_id, (channel_configuration, channel_state))| {
                        match channel_state {
                            StateDeviceEnabledChannel::EnabledQueued { queue, order_index } => {
                                if *queue >= channel_configuration.round_min {
                                    Some((channel_id, order_index))
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }
                    })
                    .sorted_by_key(|(_channel_id, order_index)| *order_index)
                    .map(|(channel_id, _order_index)| channel_id)
                    .enumerate()
                    .map(|(position, channel_id)| (channel_id, position))
                    .collect::<HashMap<_, _>>();

                // total power of active channels
                let power = zip_eq(self.configuration.channels.iter(), channels.iter())
                    .map(
                        |(channel_configuration, channel_state)| match channel_state {
                            StateDeviceEnabledChannel::EnabledActive { .. } => {
                                channel_configuration.power_required
                            }
                            _ => Multiplier::zero(),
                        },
                    )
                    .sum::<Multiplier>();

                let gui_channels = channels
                    .iter()
                    .enumerate()
                    .map(|(channel_id, channel_state)| match channel_state {
                        StateDeviceEnabledChannel::Disabled => {
                            GuiSummaryStateEnabledChannelState::Disabled
                        }
                        StateDeviceEnabledChannel::Paused { queue, .. } => {
                            GuiSummaryStateEnabledChannelState::Paused {
                                queue_seconds: queue.as_secs_f64(),
                            }
                        }
                        StateDeviceEnabledChannel::EnabledQueued { queue, .. } => {
                            GuiSummaryStateEnabledChannelState::EnabledQueued {
                                queue_seconds: queue.as_secs_f64(),
                                queue_position: queued_positions.get(&channel_id).copied(),
                            }
                        }
                        StateDeviceEnabledChannel::EnabledActive { queue, round, .. } => {
                            GuiSummaryStateEnabledChannelState::EnabledActive {
                                queue_seconds: queue.as_secs_f64(),
                                round_seconds: round.as_secs_f64(),
                            }
                        }
                    })
                    .collect();

                GuiSummaryState::Enabled {
                    channels: gui_channels,
                    power: power.to_f64(),
                }
            }
        };

        let gui_summary = GuiSummary {
            configuration: gui_summary_configuration,
            state: gui_summary_state,
        };
        gui_summary
    }
}

impl uri_cursor::Handler for Device {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Next("device", uri_cursor) => match uri_cursor.as_last() {
                Some("disable") => match *request.method() {
                    http::Method::POST => {
                        self.device_disable();
                        async move { web::Response::ok_empty() }.boxed()
                    }
                    _ => async move { web::Response::error_405() }.boxed(),
                },
                Some("pause") => match *request.method() {
                    http::Method::POST => {
                        self.device_pause();
                        async move { web::Response::ok_empty() }.boxed()
                    }
                    _ => async move { web::Response::error_405() }.boxed(),
                },
                Some("enable") => match *request.method() {
                    http::Method::POST => {
                        self.device_enable();
                        async move { web::Response::ok_empty() }.boxed()
                    }
                    _ => async move { web::Response::error_405() }.boxed(),
                },
                _ => async move { web::Response::error_404() }.boxed(),
            },
            uri_cursor::UriCursor::Next("channels", uri_cursor) => match uri_cursor.as_ref() {
                uri_cursor::UriCursor::Next("all", uri_cursor) => match uri_cursor.as_last() {
                    Some("clear") => match *request.method() {
                        http::Method::POST => {
                            self.channels_clear();
                            async move { web::Response::ok_empty() }.boxed()
                        }
                        _ => async move { web::Response::error_405() }.boxed(),
                    },
                    Some("add") => match *request.method() {
                        http::Method::POST => {
                            let multiplier = match request.body_parse_json::<Multiplier>() {
                                Ok(handler_channel_add) => handler_channel_add,
                                Err(error) => return async move {
                                    web::Response::error_400_from_error(error)
                                }
                                .boxed(),
                            };

                            self.channels_add(multiplier);
                            async move { web::Response::ok_empty() }.boxed()
                        }
                        _ => async move { web::Response::error_405() }.boxed(),
                    },
                    _ => async move { web::Response::error_404() }.boxed(),
                },
                uri_cursor::UriCursor::Next(channel_id_string, uri_cursor) => {
                    let channel_id: usize = match channel_id_string.parse().context("channel_id") {
                        Ok(channel_id) => channel_id,
                        Err(error) => {
                            return async move { web::Response::error_400_from_error(error) }
                                .boxed()
                        }
                    };
                    if !(0..self.configuration.channels.len()).contains(&channel_id) {
                        return async move { web::Response::error_404() }.boxed();
                    }

                    match uri_cursor.as_last() {
                        Some("disable") => match *request.method() {
                            http::Method::POST => {
                                self.channel_disable(channel_id);
                                async move { web::Response::ok_empty() }.boxed()
                            }
                            _ => async move { web::Response::error_405() }.boxed(),
                        },
                        Some("pause") => match *request.method() {
                            http::Method::POST => {
                                self.channel_pause(channel_id);
                                async move { web::Response::ok_empty() }.boxed()
                            }
                            _ => async move { web::Response::error_405() }.boxed(),
                        },
                        Some("enable") => match *request.method() {
                            http::Method::POST => {
                                self.channel_enable(channel_id);
                                async move { web::Response::ok_empty() }.boxed()
                            }
                            _ => async move { web::Response::error_405() }.boxed(),
                        },
                        Some("clear") => match *request.method() {
                            http::Method::POST => {
                                self.channel_clear(channel_id);
                                async move { web::Response::ok_empty() }.boxed()
                            }
                            _ => async move { web::Response::error_405() }.boxed(),
                        },
                        Some("add") => match *request.method() {
                            http::Method::POST => {
                                let multiplier = match request.body_parse_json::<Multiplier>() {
                                    Ok(handler_channel_add) => handler_channel_add,
                                    Err(error) => return async move {
                                        web::Response::error_400_from_error(error)
                                    }
                                    .boxed(),
                                };

                                self.channel_add(channel_id, multiplier);
                                async move { web::Response::ok_empty() }.boxed()
                            }
                            _ => async move { web::Response::error_405() }.boxed(),
                        },
                        Some("move-front") => match *request.method() {
                            http::Method::POST => {
                                self.channel_move_front(channel_id);
                                async move { web::Response::ok_empty() }.boxed()
                            }
                            _ => async move { web::Response::error_405() }.boxed(),
                        },
                        Some("move-back") => match *request.method() {
                            http::Method::POST => {
                                self.channel_move_back(channel_id);
                                async move { web::Response::ok_empty() }.boxed()
                            }
                            _ => async move { web::Response::error_405() }.boxed(),
                        },
                        _ => async move { web::Response::error_404() }.boxed(),
                    }
                }
                _ => async move { web::Response::error_404() }.boxed(),
            },
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
