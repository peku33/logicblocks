pub mod logic {
    use super::{super::logic::runner, hardware};
    use crate::{
        datatypes::color_rgb_boolean::ColorRgbBoolean,
        devices,
        signals::{self, signal},
        util::{
            async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
            async_flag,
            runnable::{Exited, Runnable},
        },
    };
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use futures::{future::FutureExt, join, stream::StreamExt};
    use itertools::{Itertools, chain};
    use serde::Serialize;
    use std::iter;

    #[derive(Debug)]
    pub struct DeviceFactory;
    impl runner::DeviceFactory for DeviceFactory {
        type Device<'h> = Device<'h>;

        fn new(hardware_device: &hardware::Device) -> Device<'_> {
            Device::new(hardware_device)
        }
    }

    #[derive(Debug)]
    pub struct Device<'h> {
        hardware_device: &'h hardware::Device,
        properties_remote: hardware::PropertiesRemote<'h>,

        signals_targets_changed_waker: signals::waker::TargetsChangedWaker,
        signals_sources_changed_waker: signals::waker::SourcesChangedWaker,
        signal_status_led: signal::state_target_last::Signal<hardware::StatusLedValue>,
        signal_analog_ins: [Option<signal::state_source::Signal<hardware::AnalogInValue>>;
            hardware::ANALOG_INS_COUNT],
        signal_digital_ins: [Option<signal::state_source::Signal<hardware::DigitalInValue>>;
            hardware::DIGITAL_INS_COUNT],
        signal_digital_outs: [Option<signal::state_target_last::Signal<hardware::DigitalOutValue>>;
            hardware::DIGITAL_OUTS_COUNT],
        signal_ds18x20s: [Option<signal::state_source::Signal<hardware::Ds18x20Value>>;
            hardware::DS18X20S_COUNT],

        gui_summary_waker: devices::gui_summary::Waker,
    }
    impl<'h> Device<'h> {
        pub fn new(hardware_device: &'h hardware::Device) -> Self {
            let block_functions_reversed = hardware_device.block_functions_reversed();

            Self {
                hardware_device,
                properties_remote: hardware_device.properties_remote(),

                signals_targets_changed_waker: signals::waker::TargetsChangedWaker::new(),
                signals_sources_changed_waker: signals::waker::SourcesChangedWaker::new(),
                signal_status_led:
                    signal::state_target_last::Signal::<hardware::StatusLedValue>::new(),
                signal_analog_ins: block_functions_reversed
                    .analog_in_mask
                    .iter()
                    .map(|analog_in_enabled| {
                        if *analog_in_enabled {
                            Some(signal::state_source::Signal::<hardware::AnalogInValue>::new(None))
                        } else {
                            None
                        }
                    })
                    .collect::<ArrayVec<_, { hardware::ANALOG_INS_COUNT }>>()
                    .into_inner()
                    .unwrap(),
                signal_digital_ins: block_functions_reversed
                    .digital_in_mask
                    .iter()
                    .map(|digital_in_enabled| {
                        if *digital_in_enabled {
                            Some(
                                signal::state_source::Signal::<hardware::DigitalInValue>::new(None),
                            )
                        } else {
                            None
                        }
                    })
                    .collect::<ArrayVec<_, { hardware::DIGITAL_INS_COUNT }>>()
                    .into_inner()
                    .unwrap(),
                signal_digital_outs: block_functions_reversed
                    .digital_out_mask
                    .iter()
                    .map(|digital_out_enabled| {
                        if *digital_out_enabled {
                            Some(
                                signal::state_target_last::Signal::<hardware::DigitalOutValue>::new(
                                ),
                            )
                        } else {
                            None
                        }
                    })
                    .collect::<ArrayVec<_, { hardware::DIGITAL_OUTS_COUNT }>>()
                    .into_inner()
                    .unwrap(),
                signal_ds18x20s: block_functions_reversed
                    .ds18x20_mask
                    .iter()
                    .map(|ds18x20_enabled| {
                        if *ds18x20_enabled {
                            Some(signal::state_source::Signal::<hardware::Ds18x20Value>::new(
                                None,
                            ))
                        } else {
                            None
                        }
                    })
                    .collect::<ArrayVec<_, { hardware::DS18X20S_COUNT }>>()
                    .into_inner()
                    .unwrap(),

                gui_summary_waker: devices::gui_summary::Waker::new(),
            }
        }

        fn signals_targets_changed(&self) {
            let mut properties_outs_changed = false;
            let mut gui_summary_changed = false;

            if let Some(status_led_value) = self.signal_status_led.take_pending()
                && self
                    .properties_remote
                    .status_led
                    .set(status_led_value.unwrap_or(ColorRgbBoolean::off()))
            {
                properties_outs_changed = true;
                gui_summary_changed = true;
            }

            let digital_outs_last = self
                .signal_digital_outs
                .iter()
                .map(|signal_digital_out| {
                    signal_digital_out
                        .as_ref()
                        .map(|signal_digital_out| signal_digital_out.take_last())
                })
                .collect::<ArrayVec<_, { hardware::DIGITAL_OUTS_COUNT }>>()
                .into_inner()
                .unwrap();
            if digital_outs_last.iter().any(|digital_out_last| {
                digital_out_last
                    .as_ref()
                    .map(|digital_out_last| digital_out_last.pending)
                    .unwrap_or(false)
            }) {
                let digital_outs = digital_outs_last
                    .iter()
                    .map(|digital_out_last| {
                        digital_out_last
                            .as_ref()
                            .and_then(|digital_out_last| digital_out_last.value)
                            .unwrap_or(false)
                    })
                    .collect::<ArrayVec<_, { hardware::DIGITAL_OUTS_COUNT }>>()
                    .into_inner()
                    .unwrap();

                if self.properties_remote.digital_outs.set(digital_outs) {
                    properties_outs_changed = true;
                    gui_summary_changed = true;
                }
            }

            if properties_outs_changed {
                self.properties_remote.outs_changed_waker_remote.wake();
            }
            if gui_summary_changed {
                self.gui_summary_waker.wake();
            }
        }
        fn properties_ins_changed(&self) {
            let mut signals_sources_changed = false;
            let mut gui_summary_changed = false;

            if let Some(analog_ins) = self.properties_remote.analog_ins.take_pending() {
                if let Some(analog_ins) = analog_ins {
                    self.signal_analog_ins
                        .iter()
                        .zip_eq(analog_ins)
                        .filter_map(|(signal_analog_in, analog_in_value)| {
                            signal_analog_in
                                .as_ref()
                                .map(|signal_analog_in| (signal_analog_in, analog_in_value))
                        })
                        .for_each(|(signal_analog_in, analog_in_value)| {
                            signals_sources_changed |=
                                signal_analog_in.set_one(Some(analog_in_value));
                        });
                } else {
                    self.signal_analog_ins
                        .iter()
                        .filter_map(|signal_analog_in| signal_analog_in.as_ref())
                        .for_each(|signal_analog_in| {
                            signals_sources_changed |= signal_analog_in.set_one(None);
                        });
                }

                gui_summary_changed = true;
            }

            if let Some(digital_ins) = self.properties_remote.digital_ins.take_pending() {
                if let Some(digital_ins) = digital_ins {
                    self.signal_digital_ins
                        .iter()
                        .zip_eq(digital_ins)
                        .filter_map(|(signal_digital_in, digital_in_value)| {
                            signal_digital_in
                                .as_ref()
                                .map(|signal_digital_in| (signal_digital_in, digital_in_value))
                        })
                        .for_each(|(signal_digital_in, digital_in_value)| {
                            signals_sources_changed |=
                                signal_digital_in.set_one(Some(digital_in_value));
                        });
                } else {
                    self.signal_digital_ins
                        .iter()
                        .filter_map(|signal_digital_in| signal_digital_in.as_ref())
                        .for_each(|signal_digital_in| {
                            signals_sources_changed |= signal_digital_in.set_one(None);
                        });
                }

                gui_summary_changed = true;
            }

            if let Some(ds18x20s) = self.properties_remote.ds18x20s.take_pending() {
                if let Some(ds18x20s) = ds18x20s {
                    self.signal_ds18x20s
                        .iter()
                        .zip_eq(ds18x20s)
                        .filter_map(|(signal_ds18x20, ds18x20_value)| {
                            signal_ds18x20
                                .as_ref()
                                .map(|signal_ds18x20| (signal_ds18x20, ds18x20_value))
                        })
                        .for_each(|(signal_ds18x20, ds18x20_value)| {
                            signals_sources_changed |= signal_ds18x20.set_one(Some(ds18x20_value));
                        });
                } else {
                    self.signal_ds18x20s
                        .iter()
                        .filter_map(|signal_ds18x20| signal_ds18x20.as_ref())
                        .for_each(|signal_ds18x20| {
                            signals_sources_changed |= signal_ds18x20.set_one(None);
                        });
                }

                gui_summary_changed = true;
            }

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
            // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
            let signals_targets_changed_runner = self
                .signals_targets_changed_waker
                .stream()
                .stream_take_until_exhausted(exit_flag.clone())
                .for_each(async |()| {
                    self.signals_targets_changed();
                })
                .boxed();

            // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
            let properties_ins_changed_runner = self
                .properties_remote
                .ins_changed_waker_remote
                .stream()
                .stream_take_until_exhausted(exit_flag.clone())
                .for_each(async |()| {
                    self.properties_ins_changed();
                })
                .boxed();

            let _: ((), ()) = join!(
                signals_targets_changed_runner,
                properties_ins_changed_runner
            );

            Exited
        }
    }

    impl runner::Device for Device<'_> {
        type HardwareDevice = hardware::Device;

        fn class() -> &'static str {
            "gpio_a_v1"
        }

        fn as_runnable(&self) -> &dyn Runnable {
            self
        }
        fn as_gui_summary_device_base(&self) -> Option<&dyn devices::gui_summary::DeviceBase> {
            Some(self)
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub enum SignalIdentifier {
        StatusLed,
        AnalogIn(usize),
        DigitalIn(usize),
        DigitalOut(usize),
        Ds18x20(usize),
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
            chain!(
                iter::once((
                    SignalIdentifier::StatusLed,
                    &self.signal_status_led as &dyn signal::Base,
                )),
                self.signal_analog_ins.iter().enumerate().filter_map(
                    |(analog_in_index, signal_analog_in)| {
                        signal_analog_in.as_ref().map(|signal_analog_in| {
                            (
                                SignalIdentifier::AnalogIn(analog_in_index),
                                signal_analog_in as &dyn signal::Base,
                            )
                        })
                    },
                ),
                self.signal_digital_ins.iter().enumerate().filter_map(
                    |(digital_in_index, signal_digital_in)| {
                        signal_digital_in.as_ref().map(|signal_digital_in| {
                            (
                                SignalIdentifier::DigitalIn(digital_in_index),
                                signal_digital_in as &dyn signal::Base,
                            )
                        })
                    },
                ),
                self.signal_digital_outs.iter().enumerate().filter_map(
                    |(digital_out_index, signal_digital_out)| {
                        signal_digital_out.as_ref().map(|signal_digital_out| {
                            (
                                SignalIdentifier::DigitalOut(digital_out_index),
                                signal_digital_out as &dyn signal::Base,
                            )
                        })
                    },
                ),
                self.signal_ds18x20s.iter().enumerate().filter_map(
                    |(ds18x20_index, signal_ds18x20)| {
                        signal_ds18x20.as_ref().map(|signal_ds18x20| {
                            (
                                SignalIdentifier::Ds18x20(ds18x20_index),
                                signal_ds18x20 as &dyn signal::Base,
                            )
                        })
                    },
                ),
            )
            .collect::<signals::ByIdentifier<_>>()
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

    #[derive(Debug, Serialize)]
    #[serde(tag = "function", content = "value")]
    enum GuiSummaryBlock1Value {
        Unused,
        AnalogIn(Option<hardware::AnalogInValue>),
        DigitalIn(Option<hardware::DigitalInValue>),
        DigitalOut(hardware::DigitalOutValue),
    }
    #[derive(Debug, Serialize)]
    #[serde(tag = "function", content = "value")]
    enum GuiSummaryBlock2Value {
        Unused,
        DigitalIn(Option<hardware::DigitalInValue>),
        DigitalOut(hardware::DigitalOutValue),
        Ds18x20(Option<hardware::Ds18x20Value>),
    }
    #[derive(Debug, Serialize)]
    #[serde(tag = "function", content = "value")]
    enum GuiSummaryBlock3Value {
        Unused,
        AnalogIn(Option<hardware::AnalogInValue>),
    }
    #[derive(Debug, Serialize)]
    #[serde(tag = "function", content = "value")]
    enum GuiSummaryBlock4Value {
        Unused,
        DigitalOut(hardware::DigitalOutValue),
    }
    #[derive(Debug, Serialize)]
    pub struct GuiSummary {
        status_led: hardware::StatusLedValue,
        block_1_values: [GuiSummaryBlock1Value; hardware::BLOCK_1_SIZE],
        block_2_values: [GuiSummaryBlock2Value; hardware::BLOCK_2_SIZE],
        block_3_values: [GuiSummaryBlock3Value; hardware::BLOCK_3_SIZE],
        block_4_values: [GuiSummaryBlock4Value; hardware::BLOCK_4_SIZE],
    }
    impl devices::gui_summary::Device for Device<'_> {
        fn waker(&self) -> &devices::gui_summary::Waker {
            &self.gui_summary_waker
        }

        type Value = GuiSummary;
        fn value(&self) -> Self::Value {
            let configuration = self.hardware_device.configuration();
            let block_functions = &configuration.block_functions;

            let status_led = self.properties_remote.status_led.peek_last();
            let analog_ins = self.properties_remote.analog_ins.peek_last();
            let digital_ins = self.properties_remote.digital_ins.peek_last();
            let digital_outs = self.properties_remote.digital_outs.peek_last();
            let ds18x20s = self.properties_remote.ds18x20s.peek_last();

            let block_1_values = block_functions
                .block_1_functions
                .iter()
                .enumerate()
                .map(|(index, block_1_function)| match block_1_function {
                    hardware::Block1Function::Unused => GuiSummaryBlock1Value::Unused,
                    hardware::Block1Function::AnalogIn => GuiSummaryBlock1Value::AnalogIn(
                        analog_ins.as_ref().map(|analog_ins| analog_ins[index]),
                    ),
                    hardware::Block1Function::DigitalIn => GuiSummaryBlock1Value::DigitalIn(
                        digital_ins.as_ref().map(|digital_ins| digital_ins[index]),
                    ),
                    hardware::Block1Function::DigitalOut => {
                        GuiSummaryBlock1Value::DigitalOut(digital_outs[index])
                    }
                })
                .collect::<ArrayVec<_, { hardware::BLOCK_1_SIZE }>>()
                .into_inner()
                .unwrap();
            let block_2_values = block_functions
                .block_2_functions
                .iter()
                .enumerate()
                .map(|(index, block_2_function)| match block_2_function {
                    hardware::Block2Function::Unused => GuiSummaryBlock2Value::Unused,
                    hardware::Block2Function::DigitalIn => GuiSummaryBlock2Value::DigitalIn(
                        digital_ins
                            .as_ref()
                            .map(|digital_ins| digital_ins[hardware::BLOCK_1_SIZE + index]),
                    ),
                    hardware::Block2Function::DigitalOut => GuiSummaryBlock2Value::DigitalOut(
                        digital_outs[hardware::BLOCK_1_SIZE + index],
                    ),
                    hardware::Block2Function::Ds18x20 => GuiSummaryBlock2Value::Ds18x20(
                        ds18x20s.as_ref().map(|ds18x20s| ds18x20s[index]),
                    ),
                })
                .collect::<ArrayVec<_, { hardware::BLOCK_2_SIZE }>>()
                .into_inner()
                .unwrap();
            let block_3_values = block_functions
                .block_3_functions
                .iter()
                .enumerate()
                .map(|(index, block_3_function)| match block_3_function {
                    hardware::Block3Function::Unused => GuiSummaryBlock3Value::Unused,
                    hardware::Block3Function::AnalogIn => GuiSummaryBlock3Value::AnalogIn(
                        analog_ins
                            .as_ref()
                            .map(|analog_ins| analog_ins[hardware::BLOCK_1_SIZE + index]),
                    ),
                })
                .collect::<ArrayVec<_, { hardware::BLOCK_3_SIZE }>>()
                .into_inner()
                .unwrap();
            let block_4_values = block_functions
                .block_4_functions
                .iter()
                .enumerate()
                .map(|(index, block_4_function)| match block_4_function {
                    hardware::Block4Function::Unused => GuiSummaryBlock4Value::Unused,
                    hardware::Block4Function::DigitalOut => GuiSummaryBlock4Value::DigitalOut(
                        digital_outs[hardware::BLOCK_1_SIZE + hardware::BLOCK_2_SIZE + index],
                    ),
                })
                .collect::<ArrayVec<_, { hardware::BLOCK_4_SIZE }>>()
                .into_inner()
                .unwrap();

            let gui_summary = GuiSummary {
                status_led,
                block_1_values,
                block_2_values,
                block_3_values,
                block_4_values,
            };
            gui_summary
        }
    }
}
pub mod hardware {
    use super::super::{
        super::houseblocks_v1::common::{AddressDeviceType, Payload},
        datatypes::ds18x20::State as Ds18x20State,
        hardware::{
            datatypes::ds18x20::SensorState as Ds18x20SensorState, driver::ApplicationDriver,
            parser::Parser, runner, serializer::Serializer,
        },
        properties,
    };
    use crate::{
        datatypes::{color_rgb_boolean::ColorRgbBoolean, voltage::Voltage},
        util::{
            async_ext::stream_take_until_exhausted::StreamTakeUntilExhaustedExt,
            async_flag, async_waker,
            runnable::{Exited, Runnable},
        },
    };
    use anyhow::{Context, Error, bail, ensure};
    use arrayvec::ArrayVec;
    use async_trait::async_trait;
    use futures::{future::FutureExt, join, stream::StreamExt};
    use itertools::chain;
    use std::{iter, time::Duration};

    // block configuration
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Block1Function {
        Unused,
        AnalogIn,
        DigitalIn,
        DigitalOut,
    }
    pub const BLOCK_1_SIZE: usize = 4;
    pub type Block1Functions = [Block1Function; BLOCK_1_SIZE];

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Block2Function {
        Unused,
        DigitalIn,
        DigitalOut,
        Ds18x20,
    }
    pub const BLOCK_2_SIZE: usize = 4;
    pub type Block2Functions = [Block2Function; BLOCK_2_SIZE];

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Block3Function {
        Unused,
        AnalogIn,
    }
    pub const BLOCK_3_SIZE: usize = 2;
    pub type Block3Functions = [Block3Function; BLOCK_3_SIZE];

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Block4Function {
        Unused,
        DigitalOut,
    }
    pub const BLOCK_4_SIZE: usize = 3;
    pub type Block4Functions = [Block4Function; BLOCK_4_SIZE];

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct BlockFunctions {
        pub block_1_functions: Block1Functions,
        pub block_2_functions: Block2Functions,
        pub block_3_functions: Block3Functions,
        pub block_4_functions: Block4Functions,
    }

    // block functions aggregated
    pub type StatusLedValue = ColorRgbBoolean;

    pub type AnalogInValue = Voltage;
    pub const ANALOG_INS_COUNT: usize = BLOCK_1_SIZE + BLOCK_3_SIZE;
    pub type AnalogInValues = [AnalogInValue; ANALOG_INS_COUNT];

    pub type DigitalInValue = bool;
    pub const DIGITAL_INS_COUNT: usize = BLOCK_1_SIZE + BLOCK_2_SIZE;
    pub type DigitalInValues = [DigitalInValue; DIGITAL_INS_COUNT];

    pub type DigitalOutValue = bool;
    pub const DIGITAL_OUTS_COUNT: usize = BLOCK_1_SIZE + BLOCK_2_SIZE + BLOCK_4_SIZE;
    pub type DigitalOutValues = [DigitalOutValue; DIGITAL_OUTS_COUNT];

    pub type Ds18x20Value = Ds18x20State;
    pub const DS18X20S_COUNT: usize = BLOCK_2_SIZE;
    pub type Ds18x20Values = [Ds18x20Value; DS18X20S_COUNT];

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct BlockFunctionsReversed {
        pub analog_in_mask: [bool; ANALOG_INS_COUNT],
        pub analog_in_any: bool,

        pub digital_in_mask: [bool; DIGITAL_INS_COUNT],
        pub digital_in_any: bool,

        pub digital_out_mask: [bool; DIGITAL_OUTS_COUNT],
        pub digital_out_any: bool,

        pub ds18x20_mask: [bool; DS18X20S_COUNT],
        pub ds18x20_any: bool,
    }
    impl BlockFunctionsReversed {
        pub fn new(block_functions: &BlockFunctions) -> Self {
            let analog_in_mask = chain!(
                block_functions
                    .block_1_functions
                    .iter()
                    .map(|block_1_function| *block_1_function == Block1Function::AnalogIn),
                block_functions
                    .block_3_functions
                    .iter()
                    .map(|block_3_function| *block_3_function == Block3Function::AnalogIn),
            )
            .collect::<ArrayVec<_, { ANALOG_INS_COUNT }>>()
            .into_inner()
            .unwrap();
            let analog_in_any = analog_in_mask
                .iter()
                .any(|analog_in_enabled| *analog_in_enabled);

            let digital_in_mask = chain!(
                block_functions
                    .block_1_functions
                    .iter()
                    .map(|block_1_function| *block_1_function == Block1Function::DigitalIn),
                block_functions
                    .block_2_functions
                    .iter()
                    .map(|block_2_function| *block_2_function == Block2Function::DigitalIn),
            )
            .collect::<ArrayVec<_, { DIGITAL_INS_COUNT }>>()
            .into_inner()
            .unwrap();
            let digital_in_any = digital_in_mask
                .iter()
                .any(|digital_in_enabled| *digital_in_enabled);

            let digital_out_mask = chain!(
                block_functions
                    .block_1_functions
                    .iter()
                    .map(|block_1_function| *block_1_function == Block1Function::DigitalOut),
                block_functions
                    .block_2_functions
                    .iter()
                    .map(|block_2_function| *block_2_function == Block2Function::DigitalOut),
                block_functions
                    .block_4_functions
                    .iter()
                    .map(|block_4_function| *block_4_function == Block4Function::DigitalOut),
            )
            .collect::<ArrayVec<_, { DIGITAL_OUTS_COUNT }>>()
            .into_inner()
            .unwrap();
            let digital_out_any = digital_out_mask
                .iter()
                .any(|digital_out_enabled| *digital_out_enabled);

            let ds18x20_mask = chain!(
                block_functions
                    .block_2_functions
                    .iter()
                    .map(|block_2_function| *block_2_function == Block2Function::Ds18x20),
            )
            .collect::<ArrayVec<_, { DS18X20S_COUNT }>>()
            .into_inner()
            .unwrap();
            let ds18x20_any = ds18x20_mask.iter().any(|ds18x20_enabled| *ds18x20_enabled);

            Self {
                analog_in_mask,
                analog_in_any,
                digital_in_mask,
                digital_in_any,
                digital_out_mask,
                digital_out_any,
                ds18x20_mask,
                ds18x20_any,
            }
        }
    }

    // properties
    #[derive(Debug)]
    pub struct PropertiesRemote<'p> {
        pub ins_changed_waker_remote: properties::waker::InsChangedWakerRemote<'p>,
        pub outs_changed_waker_remote: properties::waker::OutsChangedWakerRemote<'p>,

        pub status_led: properties::state_out::Remote<'p, StatusLedValue>,
        pub analog_ins: properties::state_in::Remote<'p, AnalogInValues>,
        pub digital_ins: properties::state_in::Remote<'p, DigitalInValues>,
        pub digital_outs: properties::state_out::Remote<'p, DigitalOutValues>,
        pub ds18x20s: properties::state_in::Remote<'p, Ds18x20Values>,
    }

    #[derive(Debug)]
    pub struct Properties {
        ins_changed_waker: properties::waker::InsChangedWaker,
        outs_changed_waker: properties::waker::OutsChangedWaker,

        status_led: properties::state_out::Property<StatusLedValue>,
        analog_ins: properties::state_in::Property<AnalogInValues>,
        digital_ins: properties::state_in::Property<DigitalInValues>,
        digital_outs: properties::state_out::Property<DigitalOutValues>,
        ds18x20s: properties::state_in::Property<Ds18x20Values>,
    }
    impl Properties {
        pub fn new() -> Self {
            Self {
                ins_changed_waker: properties::waker::InsChangedWaker::new(),
                outs_changed_waker: properties::waker::OutsChangedWaker::new(),

                status_led: properties::state_out::Property::<StatusLedValue>::new(
                    ColorRgbBoolean::off(),
                ),
                analog_ins: properties::state_in::Property::<AnalogInValues>::new(),
                digital_ins: properties::state_in::Property::<DigitalInValues>::new(),
                digital_outs: properties::state_out::Property::<DigitalOutValues>::new(
                    [false; DIGITAL_OUTS_COUNT],
                ),
                ds18x20s: properties::state_in::Property::<Ds18x20Values>::new(),
            }
        }

        pub fn device_reset(&self) -> bool {
            self.status_led.device_reset();
            self.digital_outs.device_reset();

            false
                || self.analog_ins.device_reset()
                || self.digital_ins.device_reset()
                || self.ds18x20s.device_reset()
        }

        pub fn remote(&self) -> PropertiesRemote<'_> {
            PropertiesRemote {
                ins_changed_waker_remote: self.ins_changed_waker.remote(),
                outs_changed_waker_remote: self.outs_changed_waker.remote(),

                status_led: self.status_led.user_remote(),
                analog_ins: self.analog_ins.user_remote(),
                digital_ins: self.digital_ins.user_remote(),
                digital_outs: self.digital_outs.user_remote(),
                ds18x20s: self.ds18x20s.user_remote(),
            }
        }
    }

    // device
    #[derive(Clone, Copy, Debug)]
    pub struct Configuration {
        pub block_functions: BlockFunctions,
    }

    #[derive(Debug)]
    pub struct Device {
        configuration: Configuration,
        block_functions_reversed: BlockFunctionsReversed,

        properties: Properties,

        poll_waker: async_waker::mpsc::Signal,
    }
    impl Device {
        pub fn new(configuration: Configuration) -> Self {
            let block_functions_reversed =
                BlockFunctionsReversed::new(&configuration.block_functions);

            Self {
                configuration,
                block_functions_reversed,

                properties: Properties::new(),

                poll_waker: async_waker::mpsc::Signal::new(),
            }
        }

        pub fn configuration(&self) -> &Configuration {
            &self.configuration
        }
        pub fn block_functions_reversed(&self) -> &BlockFunctionsReversed {
            &self.block_functions_reversed
        }
        pub fn properties_remote(&self) -> PropertiesRemote<'_> {
            self.properties.remote()
        }

        async fn run(
            &self,
            exit_flag: async_flag::Receiver,
        ) -> Exited {
            // TODO: remove .boxed() workaround for https://github.com/rust-lang/rust/issues/71723
            let properties_outs_changed_waker_runner = self
                .properties
                .outs_changed_waker
                .stream()
                .stream_take_until_exhausted(exit_flag.clone())
                .for_each(async |()| {
                    self.poll_waker.wake();
                })
                .boxed();

            let _: ((),) = join!(properties_outs_changed_waker_runner);

            Exited
        }
    }

    impl runner::Device for Device {
        fn device_type_name() -> &'static str {
            "GPIO_A_v1"
        }
        fn address_device_type() -> AddressDeviceType {
            AddressDeviceType::new_from_ordinal(5).unwrap()
        }

        fn poll_waker(&self) -> Option<&async_waker::mpsc::Signal> {
            Some(&self.poll_waker)
        }

        fn as_runnable(&self) -> Option<&dyn Runnable> {
            Some(self)
        }
    }

    #[async_trait]
    impl runner::BusDevice for Device {
        async fn initialize(
            &self,
            driver: &ApplicationDriver<'_>,
        ) -> Result<(), Error> {
            let request = BusRequest {
                configuration: Some(BusRequestConfiguration {
                    block_functions: self.configuration.block_functions,
                }),
                status_led: None,
                poll_request: false,
                analog_ins_request: false,
                digital_ins_request: false,
                digital_outs: None,
                ds18x20s_request: false,
            };
            let request_payload = request.to_payload();
            let response_payload = driver
                .transaction_out_in(request_payload, None)
                .await
                .context("transaction_out_in")?;
            let response = BusResponse::from_payload(&response_payload).context("from_payload")?;

            ensure!(
                response
                    == BusResponse {
                        poll: None,
                        analog_ins: None,
                        digital_ins: None,
                        ds18x20s: None
                    }
            );

            Ok(())
        }

        fn poll_delay(&self) -> Option<Duration> {
            // analog in, digital in -> poll in 250ms
            // ds18x20 -> poll in 1000ms
            // none -> no poll

            if false
                || self.block_functions_reversed.analog_in_any
                || self.block_functions_reversed.digital_in_any
            {
                return Some(Duration::from_millis(250));
            }
            if self.block_functions_reversed.ds18x20_any {
                return Some(Duration::from_millis(1000));
            }

            None
        }
        async fn poll(
            &self,
            driver: &ApplicationDriver<'_>,
        ) -> Result<(), Error> {
            // stage 1 - poll + read fast values
            // stage 1 request is always used, to check device uptime
            let mut stage_1_properties_ins_changed = false;

            let status_led_request = self.properties.status_led.device_pending();
            let digital_outs_request = if self.block_functions_reversed.digital_out_any {
                self.properties.digital_outs.device_pending()
            } else {
                None
            };

            let stage_1_request = BusRequest {
                configuration: None,
                status_led: status_led_request.as_ref().map(|status_led_request| {
                    BusRequestStatusLed {
                        value: **status_led_request,
                    }
                }),
                poll_request: false
                    || self.block_functions_reversed.analog_in_any
                    || self.block_functions_reversed.ds18x20_any,
                analog_ins_request: false,
                digital_ins_request: self.block_functions_reversed.digital_in_any,
                digital_outs: digital_outs_request.as_ref().map(|digital_outs_request| {
                    BusRequestDigitalOut {
                        values: **digital_outs_request,
                    }
                }),
                ds18x20s_request: false,
            };
            let stage_1_request_payload = stage_1_request.to_payload();
            let stage_1_response_payload = driver
                .transaction_out_in(stage_1_request_payload, None)
                .await
                .context("transaction_out_in stage 1")?;
            let stage_1_response = BusResponse::from_payload(&stage_1_response_payload)
                .context("from_payload stage 1")?;

            if let Some(status_led_request) = status_led_request {
                status_led_request.commit();
            }
            if let Some(digital_outs_request) = digital_outs_request {
                digital_outs_request.commit();
            }

            let BusResponse {
                poll: stage_1_response_poll,
                analog_ins: stage_1_response_analog_ins,
                digital_ins: stage_1_response_digital_ins,
                ds18x20s: stage_1_response_ds18x20s,
            } = stage_1_response;

            let stage_1_response_poll = match (stage_1_request.poll_request, stage_1_response_poll)
            {
                (false, None) => None,
                (true, Some(stage_1_response_poll)) => Some(stage_1_response_poll),
                _ => bail!("poll mismatch"),
            };
            ensure!(stage_1_response_analog_ins.is_none());
            let stage_1_response_digital_ins = match (
                stage_1_request.digital_ins_request,
                stage_1_response_digital_ins,
            ) {
                (false, None) => None,
                (true, Some(stage_1_response_digital_ins)) => Some(stage_1_response_digital_ins),
                _ => bail!("digital_ins mismatch"),
            };
            ensure!(stage_1_response_ds18x20s.is_none());

            if let Some(stage_1_response_digital_ins) = stage_1_response_digital_ins
                && self
                    .properties
                    .digital_ins
                    .device_set(stage_1_response_digital_ins.values)
            {
                stage_1_properties_ins_changed = true;
            }

            if stage_1_properties_ins_changed {
                self.properties.ins_changed_waker.wake();
            }

            // stage 2 - get additional data
            let mut stage_2_properties_ins_changed = false;

            let stage_2_request = BusRequest {
                configuration: None,
                status_led: None,
                poll_request: false,
                analog_ins_request: self.block_functions_reversed.analog_in_any
                    && (false
                        || self.properties.analog_ins.device_must_read()
                        || stage_1_response_poll
                            .as_ref()
                            .map(|stage_1_response_poll| stage_1_response_poll.analog_ins)
                            .unwrap_or(false)),
                digital_ins_request: false,
                digital_outs: None,
                ds18x20s_request: self.block_functions_reversed.ds18x20_any
                    && (false
                        || self.properties.ds18x20s.device_must_read()
                        || stage_1_response_poll
                            .as_ref()
                            .map(|stage_1_response_poll| stage_1_response_poll.ds18x20s)
                            .unwrap_or(false)),
            };
            if stage_2_request.is_nop() {
                return Ok(());
            }
            let stage_2_request_payload = stage_2_request.to_payload();
            let stage_2_response_payload = driver
                .transaction_out_in(stage_2_request_payload, None)
                .await
                .context("transaction_out_in stage 2")?;
            let stage_2_response = BusResponse::from_payload(&stage_2_response_payload)
                .context("from_payload stage 2")?;

            let BusResponse {
                poll: stage_2_response_poll,
                analog_ins: stage_2_response_analog_ins,
                digital_ins: stage_2_response_digital_ins,
                ds18x20s: stage_2_response_ds18x20s,
            } = stage_2_response;

            ensure!(stage_2_response_poll.is_none());
            let stage_2_response_analog_ins = match (
                stage_2_request.analog_ins_request,
                stage_2_response_analog_ins,
            ) {
                (false, None) => None,
                (true, Some(stage_2_response_analog_ins)) => Some(stage_2_response_analog_ins),
                _ => bail!("analog_ins mismatch"),
            };
            ensure!(stage_2_response_digital_ins.is_none());
            let stage_2_response_ds18x20s =
                match (stage_2_request.ds18x20s_request, stage_2_response_ds18x20s) {
                    (false, None) => None,
                    (true, Some(stage_2_response_ds18x20s)) => Some(stage_2_response_ds18x20s),
                    _ => bail!("ds18x20s mismatch"),
                };

            if let Some(stage_2_response_analog_ins) = stage_2_response_analog_ins
                && self
                    .properties
                    .analog_ins
                    .device_set(stage_2_response_analog_ins.values)
            {
                stage_2_properties_ins_changed = true;
            }
            if let Some(stage_2_response_ds18x20s) = stage_2_response_ds18x20s
                && self
                    .properties
                    .ds18x20s
                    .device_set(stage_2_response_ds18x20s.values)
            {
                stage_2_properties_ins_changed = true;
            }

            if stage_2_properties_ins_changed {
                self.properties.ins_changed_waker.wake();
            }

            Ok(())
        }

        async fn deinitialize(
            &self,
            _driver: &ApplicationDriver<'_>,
        ) -> Result<(), Error> {
            Ok(())
        }

        fn reset(&self) {
            if self.properties.device_reset() {
                self.properties.ins_changed_waker.wake();
            }
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

    // bus
    #[derive(PartialEq, Eq, Debug)]
    struct BusRequestConfiguration {
        pub block_functions: BlockFunctions,
    }
    impl BusRequestConfiguration {
        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            self.block_functions
                .block_1_functions
                .iter()
                .for_each(|block_1_function| {
                    let byte = match block_1_function {
                        Block1Function::Unused => b'A',
                        Block1Function::AnalogIn => b'A',
                        Block1Function::DigitalIn => b'I',
                        Block1Function::DigitalOut => b'O',
                    };
                    serializer.push_byte(byte);
                });
            self.block_functions
                .block_2_functions
                .iter()
                .for_each(|block_2_function| {
                    let byte = match block_2_function {
                        Block2Function::Unused => b'I',
                        Block2Function::DigitalIn => b'I',
                        Block2Function::DigitalOut => b'O',
                        Block2Function::Ds18x20 => b'T',
                    };
                    serializer.push_byte(byte);
                });
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusRequestStatusLed {
        pub value: StatusLedValue,
    }
    impl BusRequestStatusLed {
        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            serializer.push_bool(self.value.r);
            serializer.push_bool(self.value.g);
            serializer.push_bool(self.value.b);
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusRequestDigitalOut {
        pub values: DigitalOutValues,
    }
    impl BusRequestDigitalOut {
        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            let values = self
                .values
                .iter()
                .copied()
                .chain(iter::repeat(false))
                .take(16)
                .collect::<ArrayVec<_, 16>>()
                .into_inner()
                .unwrap();
            serializer.push_bool_array_16(values);
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusRequest {
        pub configuration: Option<BusRequestConfiguration>,
        pub status_led: Option<BusRequestStatusLed>,
        pub poll_request: bool,
        pub analog_ins_request: bool,
        pub digital_ins_request: bool,
        pub digital_outs: Option<BusRequestDigitalOut>,
        pub ds18x20s_request: bool,
    }
    impl BusRequest {
        pub fn is_nop(&self) -> bool {
            *self
                == (Self {
                    configuration: None,
                    status_led: None,
                    poll_request: false,
                    analog_ins_request: false,
                    digital_ins_request: false,
                    digital_outs: None,
                    ds18x20s_request: false,
                })
        }

        pub fn to_payload(&self) -> Payload {
            let mut serializer = Serializer::new();
            self.serialize(&mut serializer);
            serializer.into_payload()
        }

        pub fn serialize(
            &self,
            serializer: &mut Serializer,
        ) {
            if let Some(configuration) = &self.configuration {
                serializer.push_byte(b'C');
                configuration.serialize(serializer);
            }
            if let Some(status_led) = &self.status_led {
                serializer.push_byte(b'L');
                status_led.serialize(serializer);
            }
            if self.poll_request {
                serializer.push_byte(b'P');
            }
            if self.analog_ins_request {
                serializer.push_byte(b'A');
            }
            if self.digital_ins_request {
                serializer.push_byte(b'I');
            }
            if let Some(digital_outs) = &self.digital_outs {
                serializer.push_byte(b'O');
                digital_outs.serialize(serializer);
            }
            if self.ds18x20s_request {
                serializer.push_byte(b'T');
            }
        }
    }
    #[cfg(test)]
    mod tests_bus_request {
        use super::{
            super::super::super::houseblocks_v1::common::Payload, Block1Function, Block2Function,
            Block3Function, Block4Function, BlockFunctions, BusRequest, BusRequestConfiguration,
            BusRequestDigitalOut, BusRequestStatusLed, StatusLedValue,
        };

        #[test]
        fn empty() {
            let request = BusRequest {
                configuration: None,
                status_led: None,
                poll_request: false,
                analog_ins_request: false,
                digital_ins_request: false,
                digital_outs: None,
                ds18x20s_request: false,
            };
            let payload = request.to_payload();

            let payload_expected = Payload::new(Box::from(*b"")).unwrap();

            assert_eq!(payload, payload_expected);
        }

        #[test]
        fn everything() {
            let request = BusRequest {
                configuration: Some(BusRequestConfiguration {
                    block_functions: BlockFunctions {
                        block_1_functions: [
                            Block1Function::Unused,
                            Block1Function::AnalogIn,
                            Block1Function::DigitalIn,
                            Block1Function::DigitalOut,
                        ],
                        block_2_functions: [
                            Block2Function::Unused,
                            Block2Function::DigitalIn,
                            Block2Function::DigitalOut,
                            Block2Function::Ds18x20,
                        ],
                        block_3_functions: [
                            Block3Function::Unused,
                            Block3Function::AnalogIn,
                            // break
                        ],
                        block_4_functions: [
                            Block4Function::Unused,
                            Block4Function::DigitalOut,
                            Block4Function::Unused,
                        ],
                    },
                }),
                status_led: Some(BusRequestStatusLed {
                    value: StatusLedValue {
                        r: true,
                        g: true,
                        b: true,
                    },
                }),
                poll_request: true,
                analog_ins_request: true,
                digital_ins_request: true,
                digital_outs: Some(BusRequestDigitalOut {
                    values: [
                        true, true, true, true, true, true, true, true, true, true, true,
                    ],
                }),
                ds18x20s_request: true,
            };
            let payload = request.to_payload();

            let payload_expected = Payload::new(Box::from(*b"CAAIOIIOTL111PAIO07FFT")).unwrap();

            assert_eq!(payload, payload_expected);
        }

        #[test]
        fn request_1() {
            let request = BusRequest {
                configuration: None,
                status_led: Some(BusRequestStatusLed {
                    value: StatusLedValue {
                        r: false,
                        g: true,
                        b: true,
                    },
                }),
                poll_request: false,
                analog_ins_request: false,
                digital_ins_request: false,
                digital_outs: Some(BusRequestDigitalOut {
                    values: [
                        true, true, false, true, false, false, true, false, false, true, true,
                    ],
                }),
                ds18x20s_request: false,
            };
            let payload = request.to_payload();

            let payload_expected = Payload::new(Box::from(*b"L011O064B")).unwrap();

            assert_eq!(payload, payload_expected);
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponsePoll {
        pub analog_ins: bool,
        pub ds18x20s: bool,
    }
    impl BusResponsePoll {
        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let analog_ins = parser.expect_bool().context("analog_ins")?;
            let ds18x20s = parser.expect_bool().context("ds18x20s")?;
            Ok(Self {
                analog_ins,
                ds18x20s,
            })
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponseAnalogIns {
        pub values: AnalogInValues,
    }
    impl BusResponseAnalogIns {
        fn transform_block(
            value: u16,
            index: usize,
        ) -> Result<Voltage, Error> {
            // it should be 1023 actually (10 bits)
            // but for good division in tests we keep it at 1024
            ensure!((0..=1024).contains(&value));

            let multiplier = if (0..BLOCK_1_SIZE).contains(&index) {
                5.106 // divider with 1k and 47k
            } else if (BLOCK_1_SIZE..(BLOCK_1_SIZE + BLOCK_3_SIZE)).contains(&index) {
                27.727 // divider with 2.2k and 10k
            } else {
                bail!("index out of bounds");
            };

            let analog_in_value =
                Voltage::from_volts((value as f64) / 1024.0 * multiplier).unwrap();
            Ok(analog_in_value)
        }

        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let values = (0..ANALOG_INS_COUNT)
                .map(|index| -> Result<_, Error> {
                    let value = parser.expect_u16().context("expect_u16")?;
                    let value = Self::transform_block(value, index).context("transform_block")?;
                    Ok(value)
                })
                .collect::<Result<ArrayVec<_, ANALOG_INS_COUNT>, Error>>()
                .context("collect")?
                .into_inner()
                .unwrap();
            Ok(Self { values })
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponseDigitalIns {
        pub values: DigitalInValues,
    }
    impl BusResponseDigitalIns {
        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let values = parser
                .expect_bool_array_8()
                .context("expect_bool_array_8")?;
            Ok(Self { values })
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponseDs18x20s {
        pub values: Ds18x20Values,
    }
    impl BusResponseDs18x20s {
        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let values = (0..DS18X20S_COUNT)
                .map(|_ds18x20_index| {
                    Ds18x20SensorState::parse(parser)
                        .map(|ds18x20_sensor_state| ds18x20_sensor_state.into_inner())
                })
                .collect::<Result<ArrayVec<_, { DS18X20S_COUNT }>, _>>()
                .context("collect")?
                .into_inner()
                .unwrap();
            Ok(Self { values })
        }
    }

    #[derive(PartialEq, Eq, Debug)]
    struct BusResponse {
        pub poll: Option<BusResponsePoll>,
        pub analog_ins: Option<BusResponseAnalogIns>,
        pub digital_ins: Option<BusResponseDigitalIns>,
        pub ds18x20s: Option<BusResponseDs18x20s>,
    }
    impl BusResponse {
        pub fn from_payload(payload: &Payload) -> Result<Self, Error> {
            let mut parser = Parser::from_payload(payload);
            let self_ = Self::parse(&mut parser).context("parse")?;
            Ok(self_)
        }

        pub fn parse(parser: &mut Parser) -> Result<Self, Error> {
            let mut poll: Option<BusResponsePoll> = None;
            let mut analog_ins: Option<BusResponseAnalogIns> = None;
            let mut digital_ins: Option<BusResponseDigitalIns> = None;
            let mut ds18x20s: Option<BusResponseDs18x20s> = None;

            while let Some(opcode) = parser.get_byte() {
                match opcode {
                    b'P' => {
                        let value = BusResponsePoll::parse(parser).context("poll")?;
                        ensure!(poll.replace(value).is_none(), "duplicated poll");
                    }
                    b'A' => {
                        let value = BusResponseAnalogIns::parse(parser).context("analog_ins")?;
                        ensure!(analog_ins.replace(value).is_none(), "duplicated analog_ins");
                    }
                    b'I' => {
                        let value = BusResponseDigitalIns::parse(parser).context("digital_ins")?;
                        ensure!(
                            digital_ins.replace(value).is_none(),
                            "duplicated digital_ins"
                        );
                    }
                    b'T' => {
                        let value = BusResponseDs18x20s::parse(parser).context("ds18x20s")?;
                        ensure!(ds18x20s.replace(value).is_none(), "duplicated ds18x20s");
                    }
                    opcode => bail!("unrecognized opcode: {}", opcode),
                }
            }

            parser.expect_end().context("expect_end")?;

            Ok(Self {
                poll,
                analog_ins,
                digital_ins,
                ds18x20s,
            })
        }
    }
    #[cfg(test)]
    mod tests_bus_response {
        use super::{
            super::super::{
                super::houseblocks_v1::common::Payload,
                datatypes::ds18x20::{SensorType as Ds18x20SensorType, State as Ds18x20State},
            },
            BusResponse, BusResponseAnalogIns, BusResponseDigitalIns, BusResponseDs18x20s,
            BusResponsePoll,
        };
        use crate::datatypes::{
            temperature::{Temperature, Unit as TemperatureUnit},
            voltage::Voltage,
        };

        #[test]
        fn empty() {
            let payload = Payload::new(Box::from(*b"")).unwrap();
            let bus_response = BusResponse::from_payload(&payload).unwrap();

            let bus_response_expected = BusResponse {
                poll: None,
                analog_ins: None,
                digital_ins: None,
                ds18x20s: None,
            };

            assert_eq!(bus_response, bus_response_expected);
        }

        #[test]
        fn full() {
            let payload = Payload::new(Box::from(
                *b"P11A000001000200040002000400IA4T000087D09191CC90",
            ))
            .unwrap();
            let bus_response = BusResponse::from_payload(&payload).unwrap();

            let bus_response_expected = BusResponse {
                poll: Some(BusResponsePoll {
                    analog_ins: true,
                    ds18x20s: true,
                }),
                analog_ins: Some(BusResponseAnalogIns {
                    values: [
                        Voltage::from_volts(0.0).unwrap(),
                        Voltage::from_volts(1.2765).unwrap(),
                        Voltage::from_volts(2.553).unwrap(),
                        Voltage::from_volts(5.106).unwrap(),
                        Voltage::from_volts(13.8635).unwrap(),
                        Voltage::from_volts(27.727).unwrap(),
                    ],
                }),
                digital_ins: Some(BusResponseDigitalIns {
                    values: [false, false, true, false, false, true, false, true],
                }),
                ds18x20s: Some(BusResponseDs18x20s {
                    values: [
                        Ds18x20State {
                            sensor_type: Ds18x20SensorType::Empty,
                            reset_count: 0,
                            temperature: None,
                        },
                        Ds18x20State {
                            sensor_type: Ds18x20SensorType::S,
                            reset_count: 0,
                            temperature: Some(
                                Temperature::from_unit(TemperatureUnit::Celsius, 125.0).unwrap(),
                            ),
                        },
                        Ds18x20State {
                            sensor_type: Ds18x20SensorType::S,
                            reset_count: 1,
                            temperature: Some(
                                Temperature::from_unit(TemperatureUnit::Celsius, 25.0625).unwrap(),
                            ),
                        },
                        Ds18x20State {
                            sensor_type: Ds18x20SensorType::B,
                            reset_count: 0,
                            temperature: Some(
                                Temperature::from_unit(TemperatureUnit::Celsius, -55.0).unwrap(),
                            ),
                        },
                    ],
                }),
            };

            assert_eq!(bus_response, bus_response_expected);
        }
    }
}
