import { SummaryManagedBase } from "components/devices/SummaryManaged";
import { devicePostEmpty, useDeviceSummary } from "services/LogicDevicesRunner";
import Summary, { DeviceSummary } from "./Summary";

const SummaryManaged: SummaryManagedBase = (props) => {
  const { deviceId } = props;

  const deviceSummary = useDeviceSummary<DeviceSummary>(deviceId);

  const doR = (): void => {
    devicePostEmpty(deviceId, "/r");
  };
  const doS = (): void => {
    devicePostEmpty(deviceId, "/s");
  };
  const doT = (): void => {
    devicePostEmpty(deviceId, "/t");
  };

  return <Summary deviceSummary={deviceSummary} onR={doR} onS={doS} onT={doT} />;
};
export default SummaryManaged;
