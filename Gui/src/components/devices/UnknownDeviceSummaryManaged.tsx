import { SummaryManagedBase } from "./SummaryManaged";
import UnknownDeviceSummary from "./UnknownDeviceSummary";

const SummaryManaged: SummaryManagedBase = (props) => {
  return <UnknownDeviceSummary {...props} />;
};
export default SummaryManaged;
