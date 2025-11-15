use crate::{
    datatypes::{current::Current, frequency::Frequency, ratio::Ratio, voltage::Voltage},
    interfaces::modbus_rtu::{
        bus::AsyncBus,
        frames_public::{ReadHoldingRegistersRequest, WriteMultipleRegistersRequest},
    },
    util::{
        async_flag, observable,
        runnable::{Exited, Runnable},
    },
};
use anyhow::{Context, Error, bail, ensure};
use async_trait::async_trait;
use futures::{future::FutureExt, pin_mut, select};
use itertools::Itertools;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InputControl {
    pub reverse: bool,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Input {
    pub control: InputControl,
    pub speed: Ratio,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OutputRunning {
    // device status
    pub warning: Option<u16>,

    // device configuration
    pub speed_control_active: bool,

    // device control state
    pub ready: bool,
    pub running: bool,
    pub speed_actual: Ratio,
    pub reverse: bool,

    // motor configuration
    pub motor_voltage_max: Voltage,
    pub motor_current_rated: Current,
    pub motor_current_max: Current,
    pub motor_frequency_min: Frequency,
    pub motor_frequency_max: Frequency,
    pub motor_frequency_rated: Frequency,
    pub motor_speed_rated: Frequency,

    // motor control state
    pub motor_voltage: Voltage,
    pub motor_current: Current,
    pub motor_frequency: Frequency,
    pub motor_speed: Frequency,
    pub motor_torque: Ratio,
    pub motor_power: Ratio,

    // input state
    pub dc_link_voltage: Voltage,
    pub remote_input: bool,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Output {
    Initializing,
    Running(OutputRunning),
    Error,
}

#[derive(Debug)]
pub struct Device<'m> {
    bus: &'m AsyncBus,

    address: u8,

    input: observable::Value<Input>,
    output: observable::Value<Output>,
}
impl<'m> Device<'m> {
    pub fn new(
        bus: &'m AsyncBus,
        address: u8,
    ) -> Self {
        let input_default = Input {
            control: InputControl { reverse: false },
            speed: Ratio::zero(),
        };
        let input = observable::Value::new(input_default);

        let output_default = Output::Initializing;
        let output = observable::Value::new(output_default);

        Self {
            bus,

            address,

            input,
            output,
        }
    }

    pub fn input_setter(&self) -> observable::Setter<'_, Input> {
        self.input.setter()
    }
    pub fn output_getter(&self) -> observable::Getter<'_, Output> {
        self.output.getter()
    }

    const TIMEOUT_DEFAULT: Duration = Duration::from_secs(1);
    async fn modbus_read(
        &self,
        start_id: usize,
        count: usize,
    ) -> Result<Box<[u16]>, Error> {
        let request = ReadHoldingRegistersRequest::new(start_id, count).context("request")?;
        let response = self
            .bus
            .transaction(self.address, request, Self::TIMEOUT_DEFAULT)
            .await
            .context("transaction")?;
        Ok(response.into_holding_registers_values())
    }
    async fn modbus_write(
        &self,
        start_id: usize,
        words: Box<[u16]>,
    ) -> Result<(), Error> {
        let request = WriteMultipleRegistersRequest::new(start_id, words).context("request")?;
        self.bus
            .transaction(self.address, request, Self::TIMEOUT_DEFAULT)
            .await
            .context("transaction")?;
        Ok(())
    }

    async fn error_code_read(&self) -> Result<u16, Error> {
        let error_code = self
            .modbus_read(2111, 1)
            .await
            .context("modbus_read")?
            .into_iter()
            .exactly_one()
            .unwrap();

        Ok(error_code)
    }
    async fn input_reset(&self) -> Result<(), Error> {
        // reset control state to zero (control word, general control word, setpoint)
        // this unlocks the control
        self.modbus_write(2001, vec![0, 0, 0].into_boxed_slice())
            .await
            .context("modbus_write")?;

        Ok(())
    }
    async fn input_write(
        &self,
        input: &Input,
    ) -> Result<(), Error> {
        let mut control_word = 0u16;
        if input.speed > Ratio::zero() {
            control_word |= 1 << 0;
        }
        if input.control.reverse {
            control_word |= 1 << 1;
        }

        let general_control_word = 0u16;

        let speed_setpoint = (input.speed.to_f64() * 10_000.0) as u16;

        self.modbus_write(
            2001,
            vec![control_word, general_control_word, speed_setpoint].into_boxed_slice(),
        )
        .await
        .context("modbus_write")?;

        Ok(())
    }
    async fn output_read(&self) -> Result<OutputRunning, Error> {
        let (
            status_word,
            general_status_word,
            speed_actual,
            motor_frequency,
            motor_speed,
            motor_current,
            motor_torque,
            motor_power,
            motor_voltage,
            dc_link_voltage,
            error_code,
        ) = self
            .modbus_read(2101, 11)
            .await
            .context("modbus_read")?
            .into_iter()
            .collect_tuple()
            .unwrap();

        let (motor_frequency_min, motor_frequency_max) = self
            .modbus_read(101, 2)
            .await
            .context("modbus_read")?
            .into_iter()
            .collect_tuple()
            .unwrap();

        let (motor_current_max,) = self
            .modbus_read(107, 1)
            .await
            .context("modbus_read")?
            .into_iter()
            .collect_tuple()
            .unwrap();

        let (motor_voltage_max, motor_frequency_rated, motor_speed_rated, motor_current_rated) =
            self.modbus_read(110, 4)
                .await
                .context("modbus_read")?
                .into_iter()
                .collect_tuple()
                .unwrap();

        // 101 - Minimum frequency
        let motor_frequency_min = Frequency::from_hertz(motor_frequency_min as f64 / 100.0)
            .context("motor_frequency_min")?;

        // 102 - Maximum frequency
        let motor_frequency_max = Frequency::from_hertz(motor_frequency_max as f64 / 100.0)
            .context("motor_frequency_max")?;

        // 107 - Current limit
        let motor_current_max =
            Current::from_amperes(motor_current_max as f64 / 100.0).context("motor_current_max")?;

        // 110 - Motor, rated operating voltage
        let motor_voltage_max =
            Voltage::from_volts(motor_voltage_max as f64).context("motor_voltage_max")?;

        // 111 - Motor, rated frequency
        let motor_frequency_rated = Frequency::from_hertz(motor_frequency_rated as f64 / 100.0)
            .context("motor_frequency_rated")?;

        // 112 - Motor, rated speed
        let motor_speed_rated =
            Frequency::from_hertz(motor_speed_rated as f64 / 60.0).context("motor_speed_rated")?;

        // 113 - Motor, rated operational current
        let motor_current_rated = Current::from_amperes(motor_current_rated as f64 / 100.0)
            .context("motor_current_rated")?;

        // 2101 - Fieldbus status word
        let ready = status_word & (1 << 0) != 0;
        let running = status_word & (1 << 1) != 0;
        let reverse = status_word & (1 << 2) != 0;
        let fault = status_word & (1 << 3) != 0;
        let warning = status_word & (1 << 4) != 0;
        let speed_control_active = status_word & (1 << 7) != 0;

        // 2102 - Fieldbus general status word
        let remote_input = general_status_word & (1 << 11) != 0; // p3.28
        let manual_mode = general_status_word & (1 << 12) != 0; // p3.37
        let control_level_from_io = general_status_word & (1 << 13) != 0;
        let control_level_from_keypad = general_status_word & (1 << 14) != 0;
        let control_level_from_fieldbus = general_status_word & (1 << 15) != 0;

        // 2103 - Fieldbus actual speed
        let speed_actual =
            Ratio::from_f64(speed_actual as f64 / 10_000.0).context("speed_actual")?;

        // 2104 - Motor frequency
        let motor_frequency =
            Frequency::from_hertz(motor_frequency as f64 / 100.0).context("motor_frequency")?;

        // 2105 - Motor speed
        let motor_speed =
            Frequency::from_hertz(motor_speed as f64 / 60.0).context("motor_speed")?;

        // 2106 - Motor current
        let motor_current =
            Current::from_amperes(motor_current as f64 / 100.0).context("motor_current")?;

        // 2107 - Motor torque
        let motor_torque =
            Ratio::from_f64(motor_torque as f64 / 1_000.0).context("motor_torque")?;

        // 2108 - Motor power
        let motor_power = Ratio::from_f64(motor_power as f64 / 1_000.0).context("motor_power")?;

        // 2109 - Motor Voltage
        let motor_voltage =
            Voltage::from_volts(motor_voltage as f64 / 10.0).context("motor_voltage")?;

        // 2110 - DC-link voltage (DC)
        let dc_link_voltage =
            Voltage::from_volts(dc_link_voltage as f64).context("dc_link_voltage")?;

        // check for device errors
        if fault {
            ensure!(error_code > 0, "device fault without error code");
            bail!("device in fault state: {}", error_code);
        }
        let warning = if warning {
            ensure!(error_code > 0, "device warning without error code");
            Some(error_code)
        } else {
            None
        };

        // check if device has control from fieldbus
        ensure!(!control_level_from_io, "device has I/O control source");
        ensure!(
            !control_level_from_keypad,
            "device has keypad control source"
        );
        ensure!(
            control_level_from_fieldbus,
            "device has no fieldbus control source"
        );
        ensure!(!manual_mode, "device is in manual mode");

        let output_running = OutputRunning {
            warning,

            speed_control_active,

            ready,
            running,
            speed_actual,
            reverse,

            motor_voltage_max,
            motor_current_rated,
            motor_current_max,
            motor_frequency_min,
            motor_frequency_max,
            motor_frequency_rated,
            motor_speed_rated,

            motor_voltage,
            motor_current,
            motor_frequency,
            motor_speed,
            motor_torque,
            motor_power,

            dc_link_voltage,
            remote_input,
        };

        Ok(output_running)
    }

    const ERROR_53_TICK_INTERVAL: Duration = Duration::from_secs(1);
    async fn initialize(&self) -> Result<(), Error> {
        // validate device signature
        // registers 833, 836 are for some reason unreadable
        let (application_id, application_revision) = self
            .modbus_read(837, 2)
            .await
            .context("modbus_read")?
            .into_iter()
            .collect_tuple()
            .unwrap();
        ensure!(application_id == 9001, "unsupported application id");
        ensure!(
            application_revision == 38,
            "unsupported application revision"
        );

        // reset control to zeroes
        self.input_reset().await.context("input_reset")?;

        // check for error-53 (communication timeout, reset this fault)
        // check for other faults
        let mut error_code = self.error_code_read().await.context("error_code_read")?;

        if error_code == 53 {
            log::warn!("device in error 53 state, trying to reset");

            // set clearing flag
            self.modbus_write(2001, vec![(1 << 2)].into_boxed_slice())
                .await
                .context("modbus_write")?;

            // wait until error goes away
            tokio::time::sleep(Self::ERROR_53_TICK_INTERVAL).await;

            // restore the flag
            self.modbus_write(2001, vec![0].into_boxed_slice())
                .await
                .context("modbus_write")?;

            // refresh error code
            error_code = self.error_code_read().await.context("error_code_read")?;
        }

        // if the code didn't go away - fail the device
        ensure!(
            error_code == 0,
            "device is failing with error code: {}",
            error_code
        );

        Ok(())
    }

    const POLL_INTERVAL: Duration = Duration::from_millis(500);
    async fn run_once(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Result<Exited, Error> {
        // initialize
        let mut input_observer = self.input.observer(true);
        let mut output_setter = self.output.setter();

        output_setter.set(Output::Initializing);
        self.initialize().await.context("initialize")?;

        // run
        loop {
            // write input
            if let Some(input_observer_committer) = input_observer.get_changed_committer() {
                self.input_write(input_observer_committer.value())
                    .await
                    .context("input_write")?;
                input_observer_committer.commit();
            }

            // read output
            let output_running = self.output_read().await.context("output_read")?;
            output_setter.set(Output::Running(output_running));

            // prepare tick timer
            let poll_timer = tokio::time::sleep(Self::POLL_INTERVAL).fuse();
            pin_mut!(poll_timer);

            // wait for next tick or input change
            select! {
                () = input_observer.changed() => {},
                () = poll_timer => {},
                () = exit_flag => break,
            }
        }

        // finalize
        output_setter.set(Output::Initializing);

        Ok(Exited)
    }

    const ERROR_RESTART_DELAY: Duration = Duration::from_secs(5);
    async fn run(
        &self,
        mut exit_flag: async_flag::Receiver,
    ) -> Exited {
        loop {
            let error = match self.run_once(exit_flag.clone()).await.context("run_once") {
                Ok(Exited) => break,
                Err(error) => error,
            };
            log::error!("device failed: {error:?}");

            self.output.set(Output::Error);

            select! {
                () = tokio::time::sleep(Self::ERROR_RESTART_DELAY).fuse() => {},
                () = exit_flag => break,
            }
        }

        Exited
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
