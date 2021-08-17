use crate::{
    datatypes::multiplier::Multiplier,
    devices,
    signals::{self, signal},
    util::{
        async_flag,
        runtime::{Exited, Runnable},
        waker_stream,
    },
    web::{self, uri_cursor},
};
use anyhow::Context;
use async_trait::async_trait;
use core::panic;
use futures::{
    future::{BoxFuture, FutureExt},
    pin_mut, select,
    stream::StreamExt,
};
use itertools::{izip, zip_eq, Itertools};
use parking_lot::RwLock;
use serde::Serialize;
use std::{borrow::Cow, cmp::min, collections::HashMap, iter::repeat_with, time::Duration};

#[derive(Debug)]
pub struct ChannelConfiguration {
    pub name: String,

    pub base_time: Duration,
    pub power_required: Multiplier,

    pub round_min: Duration,
    pub round_max: Duration,
}

#[derive(Debug)]
pub struct Configuration {
    pub power_max: Multiplier,
    pub channels: Vec<ChannelConfiguration>,
}

#[derive(Clone, Copy, Debug)]
enum DeviceStateDisabledChannelState {
    Disabled,
    Paused,
    Enabled,
}

#[derive(Clone, Copy, Debug)]
enum DeviceStatePausedChannelState {
    Disabled,
    Paused { queue: Duration },
    Enabled { queue: Duration },
}

#[derive(Clone, Copy, Debug)]
enum DeviceStateEnabledChannelState {
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
enum DeviceState {
    Disabled {
        channels: Vec<DeviceStateDisabledChannelState>,
    },
    Paused {
        channels: Vec<DeviceStatePausedChannelState>,
    },
    Enabled {
        channels: Vec<DeviceStateEnabledChannelState>,
        order_index_last: u64,
    },
}

#[derive(Debug)]
struct State {
    device_state: DeviceState,
}

#[derive(Debug)]
pub struct Device {
    configuration: Configuration,
    state: RwLock<State>,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_add_all: signal::event_target_queued::Signal<Multiplier>,
    signal_power: signal::state_source::Signal<Multiplier>,
    signals_outputs: Vec<signal::state_source::Signal<bool>>,

    gui_summary_waker: waker_stream::mpmc::Sender,
}
impl Device {
    pub fn new(configuration: Configuration) -> Self {
        // TODO: precondition check
        // every channel must have power_required <= device power_max
        // base_time > 0
        // round_min > 0
        // round_max > 0

        let channels_count = configuration.channels.len();

        let device_state = DeviceState::Enabled {
            channels: vec![
                DeviceStateEnabledChannelState::EnabledQueued {
                    queue: Duration::ZERO,
                    order_index: 0,
                };
                channels_count
            ],
            order_index_last: 0,
        };
        let state = State { device_state };

        Self {
            configuration,
            state: RwLock::new(state),

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_add_all: signal::event_target_queued::Signal::new(),
            signal_power: signal::state_source::Signal::new(Some(Multiplier::zero())),
            signals_outputs: repeat_with(|| signal::state_source::Signal::new(Some(false)))
                .take(channels_count)
                .collect::<Vec<_>>(),

            gui_summary_waker: waker_stream::mpmc::Sender::new(),
        }
    }

    pub fn device_disable(&self) {
        let mut state = self.state.write();
        let state = &mut *state;

        let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device_state {
            DeviceState::Disabled { .. } => {}
            DeviceState::Paused { ref channels } => {
                let channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        DeviceStatePausedChannelState::Disabled => {
                            DeviceStateDisabledChannelState::Disabled
                        }
                        DeviceStatePausedChannelState::Paused { .. } => {
                            DeviceStateDisabledChannelState::Paused
                        }
                        DeviceStatePausedChannelState::Enabled { .. } => {
                            DeviceStateDisabledChannelState::Enabled
                        }
                    })
                    .collect::<Vec<_>>();

                state.device_state = DeviceState::Disabled { channels };

                gui_summary_changed = true;
            }
            DeviceState::Enabled { ref channels, .. } => {
                let channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        DeviceStateEnabledChannelState::Disabled => {
                            DeviceStateDisabledChannelState::Disabled
                        }
                        DeviceStateEnabledChannelState::Paused { .. } => {
                            DeviceStateDisabledChannelState::Paused
                        }
                        DeviceStateEnabledChannelState::EnabledQueued { .. } => {
                            DeviceStateDisabledChannelState::Enabled
                        }
                        DeviceStateEnabledChannelState::EnabledActive { .. } => {
                            // outputs and power will be zeroed at the end
                            DeviceStateDisabledChannelState::Enabled
                        }
                    })
                    .collect::<Vec<_>>();

                state.device_state = DeviceState::Disabled { channels };

                gui_summary_changed = true;
            }
        }

        // disable all channels
        signal_sources_changed |= self.signal_power.set_one(Some(Multiplier::zero()));
        for signal_output in &self.signals_outputs {
            signal_sources_changed |= signal_output.set_one(Some(false));
        }

        if signal_sources_changed {
            self.signal_sources_changed_waker.wake();
        }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }
    pub fn device_pause(&self) {
        let mut state = self.state.write();
        let state = &mut *state;

        let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device_state {
            DeviceState::Disabled { ref channels } => {
                let channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        DeviceStateDisabledChannelState::Disabled => {
                            DeviceStatePausedChannelState::Disabled
                        }
                        DeviceStateDisabledChannelState::Paused => {
                            DeviceStatePausedChannelState::Paused {
                                queue: Duration::ZERO,
                            }
                        }
                        DeviceStateDisabledChannelState::Enabled => {
                            DeviceStatePausedChannelState::Enabled {
                                queue: Duration::ZERO,
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                state.device_state = DeviceState::Paused { channels };

                gui_summary_changed = true;
            }
            DeviceState::Paused { .. } => {}
            DeviceState::Enabled { ref channels, .. } => {
                let channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        DeviceStateEnabledChannelState::Disabled => {
                            DeviceStatePausedChannelState::Disabled
                        }
                        DeviceStateEnabledChannelState::Paused { queue } => {
                            DeviceStatePausedChannelState::Paused { queue: *queue }
                        }
                        DeviceStateEnabledChannelState::EnabledQueued { queue, .. } => {
                            DeviceStatePausedChannelState::Enabled { queue: *queue }
                        }
                        DeviceStateEnabledChannelState::EnabledActive { queue, round } => {
                            // outputs and power will be zeroed at the end
                            DeviceStatePausedChannelState::Enabled {
                                queue: *queue + *round,
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                state.device_state = DeviceState::Paused { channels };

                gui_summary_changed = true;
            }
        }

        // disable all channels
        signal_sources_changed |= self.signal_power.set_one(Some(Multiplier::zero()));
        for signal_output in &self.signals_outputs {
            signal_sources_changed |= signal_output.set_one(Some(false));
        }

        if signal_sources_changed {
            self.signal_sources_changed_waker.wake();
        }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }
    pub fn device_enable(&self) {
        let mut state = self.state.write();
        let state = &mut *state;

        // let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device_state {
            DeviceState::Disabled { ref channels } => {
                let channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        DeviceStateDisabledChannelState::Disabled => {
                            DeviceStateEnabledChannelState::Disabled
                        }
                        DeviceStateDisabledChannelState::Paused => {
                            DeviceStateEnabledChannelState::Paused {
                                queue: Duration::ZERO,
                            }
                        }
                        DeviceStateDisabledChannelState::Enabled => {
                            DeviceStateEnabledChannelState::EnabledQueued {
                                queue: Duration::ZERO,
                                order_index: 0,
                            }
                        }
                    })
                    .collect::<Vec<_>>();
                state.device_state = DeviceState::Enabled {
                    channels,
                    order_index_last: 0,
                };

                gui_summary_changed = true;
            }
            DeviceState::Paused { ref channels } => {
                let channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        DeviceStatePausedChannelState::Disabled => {
                            DeviceStateEnabledChannelState::Disabled
                        }
                        DeviceStatePausedChannelState::Paused { queue } => {
                            DeviceStateEnabledChannelState::Paused { queue: *queue }
                        }
                        DeviceStatePausedChannelState::Enabled { queue } => {
                            DeviceStateEnabledChannelState::EnabledQueued {
                                queue: *queue,
                                order_index: 0,
                            }
                        }
                    })
                    .collect::<Vec<_>>();
                state.device_state = DeviceState::Enabled {
                    channels,
                    order_index_last: 0,
                };

                gui_summary_changed = true;
            }
            DeviceState::Enabled { .. } => {}
        }

        // if signal_sources_changed {
        //     self.signal_sources_changed_waker.wake();
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
        let state = &mut *state;

        let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device_state {
            DeviceState::Disabled { ref mut channels } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStateDisabledChannelState::Disabled => {}
                    DeviceStateDisabledChannelState::Paused
                    | DeviceStateDisabledChannelState::Enabled => {
                        *channel_state = DeviceStateDisabledChannelState::Disabled;

                        gui_summary_changed = true;
                    }
                }
            }
            DeviceState::Paused { ref mut channels } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStatePausedChannelState::Disabled => {}
                    DeviceStatePausedChannelState::Paused { .. }
                    | DeviceStatePausedChannelState::Enabled { .. } => {
                        *channel_state = DeviceStatePausedChannelState::Disabled;

                        gui_summary_changed = true;
                    }
                }
            }
            DeviceState::Enabled {
                ref mut channels, ..
            } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStateEnabledChannelState::Disabled => {}
                    DeviceStateEnabledChannelState::Paused { .. }
                    | DeviceStateEnabledChannelState::EnabledQueued { .. } => {
                        *channel_state = DeviceStateEnabledChannelState::Disabled;

                        gui_summary_changed = true;
                    }
                    DeviceStateEnabledChannelState::EnabledActive { .. } => {
                        *channel_state = DeviceStateEnabledChannelState::Disabled;

                        signal_sources_changed |=
                            self.signals_outputs[channel_id].set_one(Some(false));
                        signal_sources_changed |= self
                            .signal_power
                            .set_one(Some(self.power_calculate(channels)));

                        gui_summary_changed = true;
                    }
                }
            }
        }

        if signal_sources_changed {
            self.signal_sources_changed_waker.wake();
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
        let state = &mut *state;

        let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device_state {
            DeviceState::Disabled { ref mut channels } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStateDisabledChannelState::Disabled
                    | DeviceStateDisabledChannelState::Enabled => {
                        *channel_state = DeviceStateDisabledChannelState::Paused;
                        gui_summary_changed = true;
                    }
                    DeviceStateDisabledChannelState::Paused => {}
                }
            }
            DeviceState::Paused { ref mut channels } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStatePausedChannelState::Disabled => {
                        *channel_state = DeviceStatePausedChannelState::Paused {
                            queue: Duration::ZERO,
                        };
                        gui_summary_changed = true;
                    }
                    DeviceStatePausedChannelState::Enabled { queue } => {
                        *channel_state = DeviceStatePausedChannelState::Paused { queue: *queue };
                        gui_summary_changed = true;
                    }
                    DeviceStatePausedChannelState::Paused { .. } => {}
                }
            }
            DeviceState::Enabled {
                ref mut channels, ..
            } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStateEnabledChannelState::Disabled => {
                        *channel_state = DeviceStateEnabledChannelState::Paused {
                            queue: Duration::ZERO,
                        };
                        gui_summary_changed = true;
                    }
                    DeviceStateEnabledChannelState::Paused { .. } => {}
                    DeviceStateEnabledChannelState::EnabledQueued { queue, .. } => {
                        *channel_state = DeviceStateEnabledChannelState::Paused { queue: *queue };
                        gui_summary_changed = true;
                    }
                    DeviceStateEnabledChannelState::EnabledActive { queue, round, .. } => {
                        *channel_state = DeviceStateEnabledChannelState::Paused {
                            queue: *queue + *round,
                        };

                        signal_sources_changed |=
                            self.signals_outputs[channel_id].set_one(Some(false));
                        signal_sources_changed |= self
                            .signal_power
                            .set_one(Some(self.power_calculate(channels)));

                        gui_summary_changed = true;
                    }
                }
            }
        }

        if signal_sources_changed {
            self.signal_sources_changed_waker.wake();
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
        let state = &mut *state;

        // let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device_state {
            DeviceState::Disabled { ref mut channels } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStateDisabledChannelState::Disabled
                    | DeviceStateDisabledChannelState::Paused => {
                        *channel_state = DeviceStateDisabledChannelState::Enabled;
                        gui_summary_changed = true;
                    }
                    DeviceStateDisabledChannelState::Enabled => {}
                }
            }
            DeviceState::Paused { ref mut channels } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStatePausedChannelState::Disabled => {
                        *channel_state = DeviceStatePausedChannelState::Enabled {
                            queue: Duration::ZERO,
                        };
                        gui_summary_changed = true;
                    }
                    DeviceStatePausedChannelState::Paused { queue } => {
                        *channel_state = DeviceStatePausedChannelState::Enabled { queue: *queue };
                        gui_summary_changed = true;
                    }
                    DeviceStatePausedChannelState::Enabled { .. } => {}
                }
            }
            DeviceState::Enabled {
                ref mut channels,
                ref mut order_index_last,
            } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStateEnabledChannelState::Disabled => {
                        *order_index_last += 1;

                        *channel_state = DeviceStateEnabledChannelState::EnabledQueued {
                            queue: Duration::ZERO,
                            order_index: *order_index_last as i64,
                        };
                        gui_summary_changed = true;
                    }
                    DeviceStateEnabledChannelState::Paused { queue } => {
                        *order_index_last += 1;

                        *channel_state = DeviceStateEnabledChannelState::EnabledQueued {
                            queue: *queue,
                            order_index: *order_index_last as i64,
                        };
                        gui_summary_changed = true;
                    }
                    DeviceStateEnabledChannelState::EnabledQueued { .. }
                    | DeviceStateEnabledChannelState::EnabledActive { .. } => {}
                }
            }
        }

        // if signal_sources_changed {
        //     self.signal_sources_changed_waker.wake();
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
        let state = &mut *state;

        // let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device_state {
            DeviceState::Disabled { .. } => {}
            DeviceState::Paused { ref mut channels } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStatePausedChannelState::Disabled => {}
                    DeviceStatePausedChannelState::Paused { ref mut queue }
                    | DeviceStatePausedChannelState::Enabled { ref mut queue } => {
                        *queue = Duration::ZERO;
                        gui_summary_changed = true;
                    }
                }
            }
            DeviceState::Enabled {
                ref mut channels, ..
            } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStateEnabledChannelState::Disabled => {}
                    DeviceStateEnabledChannelState::Paused { ref mut queue }
                    | DeviceStateEnabledChannelState::EnabledQueued { ref mut queue, .. }
                    | DeviceStateEnabledChannelState::EnabledActive { ref mut queue, .. } => {
                        *queue = Duration::ZERO;
                        gui_summary_changed = true;
                    }
                }
            }
        }

        // if signal_sources_changed {
        //     self.signal_sources_changed_waker.wake();
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
        let state = &mut *state;

        // let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device_state {
            DeviceState::Disabled { .. } => {}
            DeviceState::Paused { ref mut channels } => {
                let channel_configuration = &self.configuration.channels[channel_id];
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStatePausedChannelState::Disabled => {}
                    DeviceStatePausedChannelState::Paused { ref mut queue }
                    | DeviceStatePausedChannelState::Enabled { ref mut queue } => {
                        *queue += channel_configuration.base_time.mul_f64(multiplier.into());
                        gui_summary_changed = true;
                    }
                }
            }
            DeviceState::Enabled {
                ref mut channels, ..
            } => {
                let channel_configuration = &self.configuration.channels[channel_id];
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStateEnabledChannelState::Disabled => {}
                    DeviceStateEnabledChannelState::Paused { ref mut queue }
                    | DeviceStateEnabledChannelState::EnabledQueued { ref mut queue, .. }
                    | DeviceStateEnabledChannelState::EnabledActive { ref mut queue, .. } => {
                        *queue += channel_configuration.base_time.mul_f64(multiplier.into());
                        gui_summary_changed = true;
                    }
                }
            }
        }

        // if signal_sources_changed {
        //     self.signal_sources_changed_waker.wake();
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
        let state = &mut *state;

        // let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device_state {
            DeviceState::Disabled { .. } => {}
            DeviceState::Paused { .. } => {}
            DeviceState::Enabled {
                ref mut channels,
                ref mut order_index_last,
            } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStateEnabledChannelState::Disabled
                    | DeviceStateEnabledChannelState::Paused { .. }
                    | DeviceStateEnabledChannelState::EnabledActive { .. } => {}
                    DeviceStateEnabledChannelState::EnabledQueued {
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
        //     self.signal_sources_changed_waker.wake();
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
        let state = &mut *state;

        let mut signal_sources_changed = false;
        let mut gui_summary_changed = false;

        match state.device_state {
            DeviceState::Disabled { .. } => {}
            DeviceState::Paused { .. } => {}
            DeviceState::Enabled {
                ref mut channels,
                ref mut order_index_last,
            } => {
                let channel_state = &mut channels[channel_id];
                match channel_state {
                    DeviceStateEnabledChannelState::Disabled
                    | DeviceStateEnabledChannelState::Paused { .. } => {}
                    DeviceStateEnabledChannelState::EnabledQueued {
                        ref mut order_index,
                        ..
                    } => {
                        *order_index_last += 1;
                        *order_index = *order_index_last as i64;

                        gui_summary_changed = true;
                    }
                    DeviceStateEnabledChannelState::EnabledActive { queue, round } => {
                        *order_index_last += 1;

                        *channel_state = DeviceStateEnabledChannelState::EnabledQueued {
                            order_index: *order_index_last as i64,
                            queue: *queue + *round,
                        };

                        signal_sources_changed |=
                            self.signals_outputs[channel_id].set_one(Some(false));
                        signal_sources_changed |= self
                            .signal_power
                            .set_one(Some(self.power_calculate(channels)));

                        gui_summary_changed = true;
                    }
                }
            }
        }

        if signal_sources_changed {
            self.signal_sources_changed_waker.wake();
        }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }

    pub fn channels_clear(&self) {
        let mut state = self.state.write();
        let state = &mut *state;

        let mut gui_summary_changed = false;

        match state.device_state {
            DeviceState::Disabled { .. } => {}
            DeviceState::Paused {
                ref mut channels, ..
            } => {
                for channel_state in channels {
                    match channel_state {
                        DeviceStatePausedChannelState::Disabled => {}
                        DeviceStatePausedChannelState::Paused { ref mut queue }
                        | DeviceStatePausedChannelState::Enabled { ref mut queue, .. } => {
                            *queue = Duration::ZERO;
                            gui_summary_changed = true;
                        }
                    }
                }
            }
            DeviceState::Enabled {
                ref mut channels, ..
            } => {
                for channel_state in channels {
                    match channel_state {
                        DeviceStateEnabledChannelState::Disabled => {}
                        DeviceStateEnabledChannelState::Paused { ref mut queue, .. }
                        | DeviceStateEnabledChannelState::EnabledQueued { ref mut queue, .. }
                        | DeviceStateEnabledChannelState::EnabledActive { ref mut queue, .. } => {
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
        let state = &mut *state;

        let mut gui_summary_changed = false;

        match state.device_state {
            DeviceState::Disabled { .. } => {}
            DeviceState::Paused {
                ref mut channels, ..
            } => {
                for (channel_configuration, channel_state) in
                    zip_eq(&self.configuration.channels, channels)
                {
                    match channel_state {
                        DeviceStatePausedChannelState::Disabled => {}
                        DeviceStatePausedChannelState::Paused { ref mut queue }
                        | DeviceStatePausedChannelState::Enabled { ref mut queue, .. } => {
                            *queue += channel_configuration.base_time.mul_f64(multiplier.into());
                            gui_summary_changed = true;
                        }
                    }
                }
            }
            DeviceState::Enabled {
                ref mut channels, ..
            } => {
                for (channel_configuration, channel_state) in
                    zip_eq(&self.configuration.channels, channels)
                {
                    match channel_state {
                        DeviceStateEnabledChannelState::Disabled => {}
                        DeviceStateEnabledChannelState::Paused { ref mut queue, .. }
                        | DeviceStateEnabledChannelState::EnabledQueued { ref mut queue, .. }
                        | DeviceStateEnabledChannelState::EnabledActive { ref mut queue, .. } => {
                            *queue += channel_configuration.base_time.mul_f64(multiplier.into());
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
        let state = &mut *state;

        // we do ticks only when device is in running state
        let (channels, order_index_last) = match state.device_state {
            DeviceState::Enabled {
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
            self.signals_outputs.iter()
        ) {
            match channel_state {
                DeviceStateEnabledChannelState::Disabled
                | DeviceStateEnabledChannelState::Paused { .. }
                | DeviceStateEnabledChannelState::EnabledQueued { .. } => {}
                DeviceStateEnabledChannelState::EnabledActive {
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

                        *channel_state = DeviceStateEnabledChannelState::EnabledQueued {
                            queue: *queue,
                            order_index: *order_index_last as i64,
                        };

                        signal_sources_changed |= signal_output.set_one(Some(false));
                    }

                    gui_summary_changed = true;
                }
            }
        }

        // in the second iteration we add new channels if they are ready to be run
        // we process and try to enable them processed by order, until first failure, to prevent starvation
        let channel_ids = zip_eq(&self.configuration.channels, channels.iter())
            .enumerate()
            .filter_map(
                |(channel_id, (channel_configuration, channel_state))| match channel_state {
                    // precondition: channel is in queued state and has enough time to start
                    DeviceStateEnabledChannelState::EnabledQueued { queue, order_index } => {
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
            .collect::<Vec<_>>();

        for channel_id in channel_ids {
            let channel_configuration = &self.configuration.channels[channel_id];
            let channel_state = &mut channels[channel_id];
            let signal_output = &self.signals_outputs[channel_id];

            match channel_state {
                // channel_ids should contain EnabledQueued only
                DeviceStateEnabledChannelState::Disabled
                | DeviceStateEnabledChannelState::Paused { .. }
                | DeviceStateEnabledChannelState::EnabledActive { .. } => panic!(),

                DeviceStateEnabledChannelState::EnabledQueued { queue, .. } => {
                    // total >= channel_configuration.round_min
                    // this precondition was checked during index preparing

                    if power_left >= channel_configuration.power_required {
                        let round = min(*queue, channel_configuration.round_max);
                        let queue = *queue - round;

                        *channel_state =
                            DeviceStateEnabledChannelState::EnabledActive { queue, round };

                        // enough power and time to start!
                        power_left -= channel_configuration.power_required;

                        signal_sources_changed |= signal_output.set_one(Some(true));
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
        signal_sources_changed |= self.signal_power.set_one(Some(power));

        if signal_sources_changed {
            self.signal_sources_changed_waker.wake();
        }
        if gui_summary_changed {
            self.gui_summary_waker.wake();
        }
    }

    fn power_calculate(
        &self,
        channels: &[DeviceStateEnabledChannelState],
    ) -> Multiplier {
        zip_eq(&self.configuration.channels, channels)
            .map(|(configuration, state)| match state {
                DeviceStateEnabledChannelState::EnabledActive { .. } => {
                    configuration.power_required
                }
                _ => Multiplier::zero(),
            })
            .sum::<Multiplier>()
    }

    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        let tick_runner = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
            Self::CHANNELS_TICK_INTERVAL,
        ))
        .for_each(async move |_| {
            self.channels_tick();
        });
        pin_mut!(tick_runner);
        let mut tick_runner = tick_runner.fuse();

        select! {
            () = tick_runner => panic!("tick_runner yielded"),
            () = exit_flag => {},
        }
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
    fn as_signals_device(&self) -> &dyn signals::Device {
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
impl signals::Device for Device {
    fn signal_targets_changed_wake(&self) {
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

    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }

    fn signals(&self) -> signals::Signals {
        std::iter::empty()
            .chain(std::array::IntoIter::new([
                &self.signal_add_all as &dyn signal::Base, // 0
                &self.signal_power as &dyn signal::Base,   // 1
            ]))
            .chain(
                // 2 + n
                self.signals_outputs
                    .iter()
                    .map(|signals_output| signals_output as &dyn signal::Base),
            )
            .enumerate()
            .map(|(signal_id, signal)| (signal_id as signals::Id, signal))
            .collect::<signals::Signals>()
    }
}

#[derive(Serialize)]
struct GuiSummaryChannelConfiguration {
    name: String,

    base_time_seconds: f64,
    power_required: f64,

    round_min_seconds: f64,
    round_max_seconds: f64,
}

#[derive(Serialize)]
struct GuiSummaryConfiguration {
    channels: Vec<GuiSummaryChannelConfiguration>,
    power_max: f64,
}

#[derive(Serialize)]
#[serde(tag = "state")]
enum GuiSummaryDeviceStateDisabledChannelState {
    Disabled,
    Paused,
    Enabled,
}
#[derive(Serialize)]
#[serde(tag = "state")]
enum GuiSummaryDeviceStatePausedChannelState {
    Disabled,
    Paused { queue_seconds: f64 },
    Enabled { queue_seconds: f64 },
}
#[derive(Serialize)]
#[serde(tag = "state")]
enum GuiSummaryDeviceStateEnabledChannelState {
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
#[derive(Serialize)]
#[serde(tag = "state")]
enum GuiSummaryState {
    Disabled {
        channels: Vec<GuiSummaryDeviceStateDisabledChannelState>,
    },
    Paused {
        channels: Vec<GuiSummaryDeviceStatePausedChannelState>,
    },
    Enabled {
        channels: Vec<GuiSummaryDeviceStateEnabledChannelState>,
        power: f64,
    },
}
#[derive(Serialize)]
struct GuiSummary {
    configuration: GuiSummaryConfiguration,
    state: GuiSummaryState,
}
impl devices::GuiSummaryProvider for Device {
    fn value(&self) -> Box<dyn devices::GuiSummary> {
        let state = self.state.read();
        let state = &*state;

        let gui_summary_configuration_channels = self
            .configuration
            .channels
            .iter()
            .map(|channel_configuration| GuiSummaryChannelConfiguration {
                name: channel_configuration.name.clone(),
                base_time_seconds: channel_configuration.base_time.as_secs_f64(),
                power_required: channel_configuration.power_required.into(),
                round_min_seconds: channel_configuration.round_min.as_secs_f64(),
                round_max_seconds: channel_configuration.round_max.as_secs_f64(),
            })
            .collect::<Vec<_>>();

        let gui_summary_configuration = GuiSummaryConfiguration {
            channels: gui_summary_configuration_channels,
            power_max: self.configuration.power_max.into(),
        };

        let gui_summary_state = match state.device_state {
            DeviceState::Disabled { ref channels } => {
                let gui_channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        DeviceStateDisabledChannelState::Disabled => {
                            GuiSummaryDeviceStateDisabledChannelState::Disabled
                        }
                        DeviceStateDisabledChannelState::Paused => {
                            GuiSummaryDeviceStateDisabledChannelState::Paused
                        }
                        DeviceStateDisabledChannelState::Enabled => {
                            GuiSummaryDeviceStateDisabledChannelState::Enabled
                        }
                    })
                    .collect::<Vec<_>>();

                GuiSummaryState::Disabled {
                    channels: gui_channels,
                }
            }
            DeviceState::Paused { ref channels, .. } => {
                let gui_channels = channels
                    .iter()
                    .map(|channel_state| match channel_state {
                        DeviceStatePausedChannelState::Disabled => {
                            GuiSummaryDeviceStatePausedChannelState::Disabled
                        }
                        DeviceStatePausedChannelState::Paused { queue } => {
                            GuiSummaryDeviceStatePausedChannelState::Paused {
                                queue_seconds: queue.as_secs_f64(),
                            }
                        }
                        DeviceStatePausedChannelState::Enabled { queue } => {
                            GuiSummaryDeviceStatePausedChannelState::Enabled {
                                queue_seconds: queue.as_secs_f64(),
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                GuiSummaryState::Paused {
                    channels: gui_channels,
                }
            }
            DeviceState::Enabled { ref channels, .. } => {
                // channel_id -> 0-based queue position (ascending)
                let queued_positions = zip_eq(&self.configuration.channels, channels.iter())
                    .enumerate()
                    .filter_map(|(channel_id, (channel_configuration, channel_state))| {
                        match channel_state {
                            DeviceStateEnabledChannelState::EnabledQueued {
                                queue,
                                order_index,
                            } => {
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
                let power = zip_eq(&self.configuration.channels, channels)
                    .map(
                        |(channel_configuration, channel_state)| match channel_state {
                            DeviceStateEnabledChannelState::EnabledActive { .. } => {
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
                        DeviceStateEnabledChannelState::Disabled => {
                            GuiSummaryDeviceStateEnabledChannelState::Disabled
                        }
                        DeviceStateEnabledChannelState::Paused { queue, .. } => {
                            GuiSummaryDeviceStateEnabledChannelState::Paused {
                                queue_seconds: queue.as_secs_f64(),
                            }
                        }
                        DeviceStateEnabledChannelState::EnabledQueued { queue, .. } => {
                            GuiSummaryDeviceStateEnabledChannelState::EnabledQueued {
                                queue_seconds: queue.as_secs_f64(),
                                queue_position: queued_positions.get(&channel_id).copied(),
                            }
                        }
                        DeviceStateEnabledChannelState::EnabledActive { queue, round, .. } => {
                            GuiSummaryDeviceStateEnabledChannelState::EnabledActive {
                                queue_seconds: queue.as_secs_f64(),
                                round_seconds: round.as_secs_f64(),
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                GuiSummaryState::Enabled {
                    channels: gui_channels,
                    power: power.into(),
                }
            }
        };

        let gui_summary = GuiSummary {
            configuration: gui_summary_configuration,
            state: gui_summary_state,
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
            uri_cursor::UriCursor::Next("channels", uri_cursor) => match &**uri_cursor {
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
