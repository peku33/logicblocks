use super::hardware;
use crate::{
    datatypes::{current::Current, frequency::Frequency, ratio::Ratio, voltage::Voltage},
    devices,
    interfaces::modbus_rtu::bus::AsyncBus,
    signals::{self, signal},
    util::{
        async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
        async_flag,
        runnable::{Exited, Runnable},
    },
};
use async_trait::async_trait;
use futures::{future::FutureExt, join, stream::StreamExt};
use maplit::hashmap;
use serde::Serialize;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Device<'m> {
    hardware_device: hardware::Device<'m>,

    signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
    signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
    signal_input_speed: signal::state_target_last::Signal<Ratio>,
    signal_input_reverse: signal::state_target_last::Signal<bool>,
    signal_output_ok: signal::state_source::Signal<bool>,
    signal_output_running: signal::state_source::Signal<bool>,

    gui_summary_waker: devices::gui_summary::Waker,
}
impl<'m> Device<'m> {
    pub fn new(
        bus: &'m AsyncBus,
        address: u8,
    ) -> Self {
        let hardware_device = hardware::Device::new(bus, address);

        Self {
            hardware_device,

            signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
            signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
            signal_input_speed: signal::state_target_last::Signal::<Ratio>::new(),
            signal_input_reverse: signal::state_target_last::Signal::<bool>::new(),
            signal_output_ok: signal::state_source::Signal::<bool>::new(Some(false)),
            signal_output_running: signal::state_source::Signal::<bool>::new(None),

            gui_summary_waker: devices::gui_summary::Waker::new(),
        }
    }

    fn signals_to_device(&self) {
        let reverse = self.signal_input_reverse.take_last().value.unwrap_or(false);
        let speed = self
            .signal_input_speed
            .take_last()
            .value
            .unwrap_or_else(Ratio::zero);

        let control = hardware::InputControl { reverse };
        let input = hardware::Input { control, speed };

        if self.hardware_device.input_setter().set(input) {
            self.gui_summary_waker.wake();
        }
    }
    fn device_to_signals(&self) {
        let mut signals_sources_changed = false;

        let output = self.hardware_device.output_getter().get();

        let output_ok = match output {
            hardware::Output::Running(output_running) => {
                output_running.warning.is_none() && output_running.ready
            }
            hardware::Output::Initializing | hardware::Output::Error => false,
        };
        signals_sources_changed |= self.signal_output_ok.set_one(Some(output_ok));

        let output_running = match output {
            hardware::Output::Running(output_running) => Some(output_running.running),
            hardware::Output::Initializing | hardware::Output::Error => None,
        };
        signals_sources_changed |= self.signal_output_running.set_one(output_running);

        if signals_sources_changed {
            self.signals_sources_changed_waker.wake();
        }
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
        let input_runner = futures::stream::once(async {})
            .chain(self.signals_targets_changed_waker.stream())
            .stream_take_until_exhausted(exit_flag.clone())
            .for_each(async |()| {
                self.signals_to_device();
            })
            .boxed();

        // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
        let output_runner = self
            .hardware_device
            .output_getter()
            .changed_stream(true)
            .stream_take_until_exhausted(exit_flag.clone())
            .for_each(async |_output| {
                self.device_to_signals();
                self.gui_summary_waker.wake();
            })
            .boxed();

        let hardware_runner = self.hardware_device.run(exit_flag.clone());

        let _: ((), (), Exited) = join!(input_runner, output_runner, hardware_runner);

        Exited
    }
}

impl devices::Device for Device<'_> {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("eaton/mmax_a")
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
impl Runnable for Device<'_> {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SignalIdentifier {
    InputSpeed,
    InputReverse,
    OutputOk,
    OutputRunning,
}
impl signals::Identifier for SignalIdentifier {}
impl signals::Device for Device<'_> {
    fn targets_changed_waker(&self) -> Option<&signals::waker::TargetsChangedWaker> {
        Some(&self.signals_targets_changed_waker)
    }
    fn sources_changed_waker(&self) -> Option<&signals::waker::SourcesChangedWaker> {
        Some(&self.signals_sources_changed_waker)
    }

    type Identifier = SignalIdentifier;
    fn by_identifier(&self) -> signals::ByIdentifier<'_, Self::Identifier> {
        hashmap! {
            SignalIdentifier::InputSpeed => &self.signal_input_speed as &dyn signal::Base,
            SignalIdentifier::InputReverse => &self.signal_input_reverse as &dyn signal::Base,
            SignalIdentifier::OutputOk => &self.signal_output_ok as &dyn signal::Base,
            SignalIdentifier::OutputRunning => &self.signal_output_running as &dyn signal::Base,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(tag = "state")]
pub enum GuiSummary {
    Initializing,
    Running {
        warning: Option<u16>,

        speed_control_active: bool,

        ready: bool,
        running: bool,
        speed_setpoint: Ratio,
        speed_actual: Ratio,
        reverse: bool,

        motor_voltage_max: Voltage,
        motor_current_rated: Current,
        motor_current_max: Current,
        motor_frequency_min: Frequency,
        motor_frequency_max: Frequency,
        motor_frequency_rated: Frequency,
        motor_speed_rated: Frequency,

        motor_voltage: Voltage,
        motor_current: Current,
        motor_frequency: Frequency,
        motor_speed: Frequency,
        motor_torque: Ratio,
        motor_power: Ratio,

        dc_link_voltage: Voltage,
        remote_input: bool,
    },
    Error,
}
impl devices::gui_summary::Device for Device<'_> {
    fn waker(&self) -> &devices::gui_summary::Waker {
        &self.gui_summary_waker
    }

    type Value = GuiSummary;
    fn value(&self) -> Self::Value {
        let input = self.hardware_device.input_setter().get();
        let output = self.hardware_device.output_getter().get();

        let gui_summary = match output {
            hardware::Output::Initializing => GuiSummary::Initializing,
            hardware::Output::Running(output_running) => GuiSummary::Running {
                warning: output_running.warning,

                speed_control_active: output_running.speed_control_active,

                ready: output_running.ready,
                running: output_running.running,
                reverse: output_running.reverse,
                speed_setpoint: input.speed,
                speed_actual: output_running.speed_actual,

                motor_voltage_max: output_running.motor_voltage_max,
                motor_current_rated: output_running.motor_current_rated,
                motor_current_max: output_running.motor_current_max,
                motor_frequency_min: output_running.motor_frequency_min,
                motor_frequency_max: output_running.motor_frequency_max,
                motor_frequency_rated: output_running.motor_frequency_rated,
                motor_speed_rated: output_running.motor_speed_rated,

                motor_voltage: output_running.motor_voltage,
                motor_current: output_running.motor_current,
                motor_frequency: output_running.motor_frequency,
                motor_speed: output_running.motor_speed,
                motor_torque: output_running.motor_torque,
                motor_power: output_running.motor_power,

                dc_link_voltage: output_running.dc_link_voltage,
                remote_input: output_running.remote_input,
            },
            hardware::Output::Error => GuiSummary::Error,
        };
        gui_summary
    }
}
