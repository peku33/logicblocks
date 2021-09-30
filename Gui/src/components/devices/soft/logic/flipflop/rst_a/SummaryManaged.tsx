import { SummaryManagedBase } from "components/devices/SummaryManaged";
import { useCallback } from "react";
import { devicePostEmpty, useDeviceSummary } from "services/LogicDevicesRunner";
import Summary, { DeviceSummary } from "./Summary";

const SummaryManaged: SummaryManagedBase = (props) => {
  const { deviceId } = props;

  const deviceSummary = useDeviceSummary<DeviceSummary>(deviceId);

  const onR = useCallback((): void => {
    devicePostEmpty(deviceId, "/r");
  }, [deviceId]);
  const onS = useCallback((): void => {
    devicePostEmpty(deviceId, "/s");
  }, [deviceId]);
  const onT = useCallback((): void => {
    devicePostEmpty(deviceId, "/t");
  }, [deviceId]);

  return <Summary deviceSummary={deviceSummary} onR={onR} onS={onS} onT={onT} />;
};
export default SummaryManaged;
