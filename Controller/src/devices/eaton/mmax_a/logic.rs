use super::hardware;
use crate::{
    datatypes::{ratio::Ratio, real::Real},
    devices,
    interfaces::modbus_rtu::bus::AsyncBus,
    signals::{self, signal},
    util::{
        async_flag,
        runtime::{Exited, Runnable},
        waker_stream,
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

    gui_summary_waker: waker_stream::mpmc::Sender,

    signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver,
    signal_input_speed: signal::state_target_last::Signal<Ratio>,
    signal_input_reverse: signal::state_target_last::Signal<bool>,
    signal_output_ok: signal::state_source::Signal<bool>,
}
impl<'m> Device<'m> {
    pub fn new(
        bus: &'m AsyncBus,
        address: u8,
    ) -> Self {
        let hardware_device = hardware::Device::new(bus, address);

        Self {
            hardware_device,

            gui_summary_waker: waker_stream::mpmc::Sender::new(),

            signal_sources_changed_waker: waker_stream::mpsc::SenderReceiver::new(),
            signal_input_speed: signal::state_target_last::Signal::<Ratio>::new(),
            signal_input_reverse: signal::state_target_last::Signal::<bool>::new(),
            signal_output_ok: signal::state_source::Signal::<bool>::new(None),
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
        let output = self.hardware_device.output_getter().get();

        let mut signal_sources_changed = false;

        signal_sources_changed |= self.signal_output_ok.set_one(
            if let hardware::Output::Running(output_running) = output {
                Some(output_running.warning.is_none() && output_running.ready)
            } else {
                Some(false)
            },
        );

        if signal_sources_changed {
            self.signal_sources_changed_waker.wake();
        }
    }

    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        // TODO: convert take_until to something like "take_until_non_empty_async_flag"
        // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
        let mut output_receiver = self
            .hardware_device
            .output_getter()
            .changed_stream(true)
            .take_until(exit_flag.clone());
        let output_runner = output_receiver
            .by_ref()
            .for_each(async move |_output| {
                self.device_to_signals();
                self.gui_summary_waker.wake();
            })
            .boxed();

        let hardware_runner = self.hardware_device.run(exit_flag.clone());

        let _: ((), Exited) = join!(output_runner, hardware_runner);

        assert!(output_receiver.is_stopped());

        Exited
    }
}
impl<'m> devices::Device for Device<'m> {
    fn class(&self) -> Cow<'static, str> {
        Cow::from("eaton/mmax_a")
    }

    fn as_signals_device(&self) -> &dyn signals::Device {
        self
    }
    fn as_runnable(&self) -> Option<&dyn Runnable> {
        Some(self)
    }
    fn as_gui_summary_provider(&self) -> Option<&dyn devices::GuiSummaryProvider> {
        Some(self)
    }
}
#[async_trait]
impl<'m> Runnable for Device<'m> {
    async fn run(
        &self,
        exit_flag: async_flag::Receiver,
    ) -> Exited {
        self.run(exit_flag).await
    }
}
impl<'m> signals::Device for Device<'m> {
    fn signal_targets_changed_wake(&self) {
        self.signals_to_device();
    }
    fn signal_sources_changed_waker_receiver(&self) -> waker_stream::mpsc::ReceiverLease {
        self.signal_sources_changed_waker.receiver()
    }
    fn signals(&self) -> signals::Signals {
        hashmap! {
            0 => &self.signal_input_speed as &dyn signal::Base,
            1 => &self.signal_input_reverse as &dyn signal::Base,
            2 => &self.signal_output_ok as &dyn signal::Base,
        }
    }
}
#[derive(Copy, Clone, Debug, Serialize)]
#[serde(tag = "state")]
enum GuiSummary {
    Initializing,
    Running {
        warning: Option<u16>,

        speed_control_active: bool,

        ready: bool,
        running: bool,
        speed_setpoint: Ratio,
        speed_actual: Ratio,
        reverse: bool,

        motor_voltage_max_v: Real,
        motor_current_rated_a: Real,
        motor_current_max_a: Real,
        motor_frequency_min_hz: Real,
        motor_frequency_max_hz: Real,
        motor_frequency_rated_hz: Real,
        motor_speed_rated_rpm: Real,

        motor_voltage_v: Real,
        motor_current_a: Real,
        motor_frequency_hz: Real,
        motor_speed_rpm: Real,
        motor_torque: Ratio,
        motor_power: Ratio,

        dc_link_voltage_v: Real,
        remote_input: bool,
    },
    Error,
}
impl<'m> devices::GuiSummaryProvider for Device<'m> {
    fn value(&self) -> Box<dyn devices::GuiSummary> {
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

                motor_voltage_max_v: output_running.motor_voltage_max_v,
                motor_current_rated_a: output_running.motor_current_rated_a,
                motor_current_max_a: output_running.motor_current_max_a,
                motor_frequency_min_hz: output_running.motor_frequency_min_hz,
                motor_frequency_max_hz: output_running.motor_frequency_max_hz,
                motor_frequency_rated_hz: output_running.motor_frequency_rated_hz,
                motor_speed_rated_rpm: output_running.motor_speed_rated_rpm,

                motor_voltage_v: output_running.motor_voltage_v,
                motor_current_a: output_running.motor_current_a,
                motor_frequency_hz: output_running.motor_frequency_hz,
                motor_speed_rpm: output_running.motor_speed_rpm,
                motor_torque: output_running.motor_torque,
                motor_power: output_running.motor_power,

                dc_link_voltage_v: output_running.dc_link_voltage_v,
                remote_input: output_running.remote_input,
            },
            hardware::Output::Error => GuiSummary::Error,
        };
        Box::new(gui_summary)
    }
    fn waker(&self) -> waker_stream::mpmc::ReceiverFactory {
        self.gui_summary_waker.receiver_factory()
    }
}
