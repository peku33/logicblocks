import dahuaIpcHdw4631cA from "./dahua/ipc_hdw4631c_a/SummaryManaged";
import eatonMmaxA from "./eaton/mmax_a/SummaryManaged";
import hikvisionDs2cd2x32xX from "./hikvision/ds2cd2x32x_x/SummaryManaged";
import houseblocksAvrV1D0003JunctionBoxMinimalV1 from "./houseblocks/avr_v1/d0003_junction_box_minimal_v1/SummaryManaged";
import houseblocksAvrV1D0006Relay14OptoAV1 from "./houseblocks/avr_v1/d0006_relay14_opto_a_v1/SummaryManaged";
import houseblocksAvrV1D0007Relay14SSRAV2 from "./houseblocks/avr_v1/d0007_relay14_ssr_a_v2/SummaryManaged";
import softLogicFlipflopRSTASummary from "./soft/logic/flipflop/rst_a/SummaryManaged";
import softTimeSequenceParallelASummary from "./soft/time/sequence_parallel_a/SummaryManaged";
import softWebButtonEventA from "./soft/web/button_event_a/SummaryManaged";
import softWebRatioSliderA from "./soft/web/ratio_slider_a/SummaryManaged";
import { SummaryManagedBase } from "./SummaryManaged";
import UnknownDeviceSummaryManaged from "./UnknownDeviceSummaryManaged";

export function getByClass(cls: string): SummaryManagedBase {
  switch (cls) {
    case "dahua/ipc_hdw4631c_a":
      return dahuaIpcHdw4631cA;
    case "eaton/mmax_a":
      return eatonMmaxA;
    case "hikvision/ds2cd2x32x_x":
      return hikvisionDs2cd2x32xX;
    case "houseblocks/avr_v1/junction_box_minimal_v1":
      return houseblocksAvrV1D0003JunctionBoxMinimalV1;
    case "houseblocks/avr_v1/relay14_opto_a_v1":
      return houseblocksAvrV1D0006Relay14OptoAV1;
    case "houseblocks/avr_v1/relay14_ssr_a_v2":
      return houseblocksAvrV1D0007Relay14SSRAV2;
    case "soft/logic/flipflop/rst_a":
      return softLogicFlipflopRSTASummary;
    case "soft/time/sequence_parallel_a":
      return softTimeSequenceParallelASummary;
    case "soft/web/button_event_a":
      return softWebButtonEventA;
    case "soft/web/ratio_slider_a":
      return softWebRatioSliderA;
  }
  return UnknownDeviceSummaryManaged;
}
