import { DeviceSummaryManaged } from "./DeviceSummaryManaged";
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
import softWebButtonEventA from "./soft/web/button_event_a/SummaryManaged";
import softWebButtonEventBooleanA from "./soft/web/button_event_boolean_a/SummaryManaged";
import softWebButtonStateMonostableA from "./soft/web/button_state_monostable_a/SummaryManaged";
import softWebRatioSliderA from "./soft/web/ratio_slider_a/SummaryManaged";
import UnknownDevice from "./UnknownDeviceSummaryManaged";

export function getByClass(class_: string): DeviceSummaryManaged {
  switch (class_) {
    case "dahua/ipc_a":
      return dahuaIpcA;
    case "eaton/mmax_a":
      return eatonMmaxA;
    case "hikvision/ds2cd2x32x_x":
      return hikvisionDs2cd2x32xX;
    case "houseblocks/avr_v1/reed_switch_v1":
      return houseblocksAvrV1D0002ReedSwitchV1;
    case "houseblocks/avr_v1/junction_box_minimal_v1":
      return houseblocksAvrV1D0003JunctionBoxMinimalV1;
    case "houseblocks/avr_v1/gpio_a_v1":
      return houseblocksAvrV1D0005GpioAV1;
    case "houseblocks/avr_v1/relay14_opto_a_v1":
      return houseblocksAvrV1D0006Relay14OptoAV1;
    case "houseblocks/avr_v1/relay14_ssr_a_v2":
      return houseblocksAvrV1D0007Relay14SSRAV2;
    case "soft/calendar/solar_position_a":
      return softCalendarSolarPositionA;
    case "soft/logic/boolean/flip_flop/override_a":
      return softLogicBooleanFlipFlopOverrideA;
    case "soft/logic/boolean/flip_flop/rst_a":
      return softLogicBooleanFlipFlopRSTA;
    case "soft/time/sequence_parallel_a":
      return softTimeSequenceParallelA;
    case "soft/web/button_event_a":
      return softWebButtonEventA;
    case "soft/web/button_event_boolean_a":
      return softWebButtonEventBooleanA;
    case "soft/web/button_state_monostable_a":
      return softWebButtonStateMonostableA;
    case "soft/web/ratio_slider_a":
      return softWebRatioSliderA;
  }
  return UnknownDevice;
}
