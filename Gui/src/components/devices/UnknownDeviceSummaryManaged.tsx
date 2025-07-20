import { type DeviceSummaryManaged } from "./DeviceSummaryManaged";
import UnknownDeviceSummary from "./UnknownDeviceSummary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  return <UnknownDeviceSummary deviceId={deviceId} />;
};
export default ManagedComponent;
