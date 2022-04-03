import { ComponentManagedBase } from "./SummaryManaged";
import UnknownDeviceSummary from "./UnknownDeviceSummary";

const ComponentManaged: ComponentManagedBase = (props) => {
  return <UnknownDeviceSummary {...props} />;
};
export default ComponentManaged;
