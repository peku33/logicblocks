import houseblocksAvrV1D0003JunctionBoxMinimalV1 from "./houseblocks/avr_v1/d0003_junction_box_minimal_v1/Summary";
import softLogicFlipflopRSTASummary from "./soft/logic/flipflop/rst_a/Summary";
import softWebButtonEventA from "./soft/web/button_event_a/Summary";
import UnknownSummary from "./UnknownSummary";

type SummaryComponent = React.FC<{
  deviceId: number;
  deviceClass: string;
}>;

export function getSummaryComponent(cls: string): SummaryComponent {
  switch (cls) {
    case "houseblocks/avr_v1/junction_box_minimal_v1":
      return houseblocksAvrV1D0003JunctionBoxMinimalV1;
    case "soft/logic/flipflop/rst_a":
      return softLogicFlipflopRSTASummary;
    case "soft/web/button_event_a":
      return softWebButtonEventA;
  }
  return UnknownSummary;
}
