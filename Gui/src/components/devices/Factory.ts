import softRSTASummary from "./soft/rst_a/Summary";
import UnknownSummary from "./UnknownSummary";

type SummaryComponent = React.FC<{
  deviceId: number;
  deviceClass: string;
}>;

export function getSummaryComponent(cls: string): SummaryComponent {
  switch (cls) {
    case "soft/rst_a":
      return softRSTASummary;
  }
  return UnknownSummary;
}
