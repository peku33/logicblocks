import { SummaryManagedBase } from "components/devices/SummaryManaged";
import { useCallback } from "react";
import { devicePostEmpty, useDeviceSummary } from "services/LogicDevicesRunner";
import Summary, { DeviceSummary } from "./Summary";

const SummaryManaged: SummaryManagedBase = (props) => {
  const { deviceId } = props;

  const deviceSummary = useDeviceSummary<DeviceSummary>(deviceId);

  const doR = useCallback((): void => {
    devicePostEmpty(deviceId, "/r");
  }, [deviceId]);
  const doS = useCallback((): void => {
    devicePostEmpty(deviceId, "/s");
  }, [deviceId]);
  const doT = useCallback((): void => {
    devicePostEmpty(deviceId, "/t");
  }, [deviceId]);

  return <Summary deviceSummary={deviceSummary} onR={doR} onS={doS} onT={doT} />;
};
export default SummaryManaged;
