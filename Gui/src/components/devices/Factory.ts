import dahuaIpcHdw4631cA from "./dahua/ipc_hdw4631c_a/Summary";
import hikvisionDs2cd2x32xX from "./hikvision/ds2cd2x32x_x/Summary";
import houseblocksAvrV1D0003JunctionBoxMinimalV1 from "./houseblocks/avr_v1/d0003_junction_box_minimal_v1/Summary";
import houseblocksAvrV1D0006Relay14OptoAV1 from "./houseblocks/avr_v1/d0006_relay14_opto_a_v1/Summary";
import houseblocksAvrV1D0007Relay14SSRAV2 from "./houseblocks/avr_v1/d0007_relay14_ssr_a_v2/Summary";
import softLogicFlipflopRSTASummary from "./soft/logic/flipflop/rst_a/Summary";
import softWebButtonEventA from "./soft/web/button_event_a/Summary";
import UnknownSummary from "./UnknownSummary";

type SummaryComponent = React.FC<{
  deviceId: number;
  deviceClass: string;
}>;

export function getSummaryComponent(cls: string): SummaryComponent {
  switch (cls) {
    case "dahua/ipc_hdw4631c_a":
      return dahuaIpcHdw4631cA;
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
    case "soft/web/button_event_a":
      return softWebButtonEventA;
  }
  return UnknownSummary;
}
