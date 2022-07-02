import { DeviceSummaryManaged } from "./DeviceSummaryManaged";
import UnknownDeviceSummary from "./UnknownDeviceSummary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceSummaryContext } = props;

  return <UnknownDeviceSummary deviceSummaryContext={deviceSummaryContext} />;
};
export default ManagedComponent;
