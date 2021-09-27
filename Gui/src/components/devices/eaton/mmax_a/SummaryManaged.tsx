import { SummaryManagedBase } from "components/devices/SummaryManaged";
import { useDeviceSummary } from "services/LogicDevicesRunner";
import Summary, { DeviceSummary } from "./Summary";

const SummaryManaged: SummaryManagedBase = (props) => {
  const { deviceId } = props;

  const deviceSummary = useDeviceSummary<DeviceSummary>(deviceId);

  return <Summary deviceSummary={deviceSummary} />;
};
export default SummaryManaged;
