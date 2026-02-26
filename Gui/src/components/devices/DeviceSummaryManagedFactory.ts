import { type DeviceSummaryManaged } from "./DeviceSummaryManaged";
//
import dahuaIpcA from "./dahua/ipc_a/SummaryManaged";
import eatonMmaxA from "./eaton/mmax_a/SummaryManaged";
import hikvisionDs2cd2x32xX from "./hikvision/ds2cd2x32x_x/SummaryManaged";
import houseblocksAvrV1D0002ReedSwitchV1 from "./houseblocks/avr_v1/d0002_reed_switch_v1/SummaryManaged";
import houseblocksAvrV1D0003JunctionBoxMinimalV1 from "./houseblocks/avr_v1/d0003_junction_box_minimal_v1/SummaryManaged";
import houseblocksAvrV1D0005GpioAV1 from "./houseblocks/avr_v1/d0005_gpio_a_v1/SummaryManaged";
import houseblocksAvrV1D0006Relay14OptoAV1 from "./houseblocks/avr_v1/d0006_relay14_opto_a_v1/SummaryManaged";
import houseblocksAvrV1D0007Relay14SSRAV2 from "./houseblocks/avr_v1/d0007_relay14_ssr_a_v2/SummaryManaged";
import softCalendarSolarPositionA from "./soft/calendar/solar_position_a/SummaryManaged";
import softLogicBooleanFlipFlopOverrideA from "./soft/logic/boolean/flip_flop/override_a/SummaryManaged";
import softLogicBooleanFlipFlopRSTA from "./soft/logic/boolean/flip_flop/rst_a/SummaryManaged";
import softTimeSequenceParallelA from "./soft/time/sequence_parallel_a/SummaryManaged";
import softTimeUpDownA from "./soft/time/up_down_a/SummaryManaged";
import softWebDisplayBooleanA from "./soft/web/display/boolean_a/SummaryManaged";
import softWebDisplayBuildingWindowOpenStateOpenClosedA from "./soft/web/display/building/window_open_state_open_closed_a/SummaryManaged";
import softWebDisplayBuildingWindowOpenStateOpenTiltedClosedA from "./soft/web/display/building/window_open_state_open_tilted_closed_a/SummaryManaged";
import softWebDisplayFlowVolumetricA from "./soft/web/display/flow_volumetric_a/SummaryManaged";
import softWebDisplayPressureA from "./soft/web/display/pressure_a/SummaryManaged";
import softWebInputBuildingButtonBlindsUpDownA from "./soft/web/input/building/button_blinds_up_down_a/SummaryManaged";
import softWebInputButtonEventA from "./soft/web/input/button_event_a/SummaryManaged";
import softWebInputButtonEventBooleanA from "./soft/web/input/button_event_boolean_a/SummaryManaged";
import softWebInputButtonStateMonostableA from "./soft/web/input/button_state_monostable_a/SummaryManaged";
import softWebInputRatioSliderA from "./soft/web/input/ratio_slider_a/SummaryManaged";
import softWebInputRealInputA from "./soft/web/input/real_input_a/SummaryManaged";
import UnknownDevice from "./UnknownDeviceSummaryManaged";

const BY_CLASS: Record<string, DeviceSummaryManaged> = {
  "dahua/ipc_a": dahuaIpcA,
  "eaton/mmax_a": eatonMmaxA,
  "hikvision/ds2cd2x32x_x": hikvisionDs2cd2x32xX,
  "houseblocks/avr_v1/gpio_a_v1": houseblocksAvrV1D0005GpioAV1,
  "houseblocks/avr_v1/junction_box_minimal_v1": houseblocksAvrV1D0003JunctionBoxMinimalV1,
  "houseblocks/avr_v1/reed_switch_v1": houseblocksAvrV1D0002ReedSwitchV1,
  "houseblocks/avr_v1/relay14_opto_a_v1": houseblocksAvrV1D0006Relay14OptoAV1,
  "houseblocks/avr_v1/relay14_ssr_a_v2": houseblocksAvrV1D0007Relay14SSRAV2,
  "soft/time/up_down_a": softTimeUpDownA,
  "soft/calendar/solar_position_a": softCalendarSolarPositionA,
  "soft/logic/boolean/flip_flop/override_a": softLogicBooleanFlipFlopOverrideA,
  "soft/logic/boolean/flip_flop/rst_a": softLogicBooleanFlipFlopRSTA,
  "soft/time/sequence_parallel_a": softTimeSequenceParallelA,
  "soft/web/display/boolean_a": softWebDisplayBooleanA,
  "soft/web/display/building/window_open_state_open_closed_a": softWebDisplayBuildingWindowOpenStateOpenClosedA,
  "soft/web/display/building/window_open_state_open_tilted_closed_a":
    softWebDisplayBuildingWindowOpenStateOpenTiltedClosedA,
  "soft/web/display/flow_volumetric_a": softWebDisplayFlowVolumetricA,
  "soft/web/display/pressure_a": softWebDisplayPressureA,
  "soft/web/input/building/button_blinds_up_down_a": softWebInputBuildingButtonBlindsUpDownA,
  "soft/web/input/button_event_a": softWebInputButtonEventA,
  "soft/web/input/button_event_boolean_a": softWebInputButtonEventBooleanA,
  "soft/web/input/button_state_monostable_a": softWebInputButtonStateMonostableA,
  "soft/web/input/ratio_slider_a": softWebInputRatioSliderA,
  "soft/web/input/real_input_a": softWebInputRealInputA,
};

export function getByClass(class_: string): DeviceSummaryManaged {
  const byClass = BY_CLASS[class_];
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
  return byClass !== undefined ? byClass : UnknownDevice;
}
