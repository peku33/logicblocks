import { deviceClassPostEmpty } from "components/devices/Device";
import { useDeviceSummary } from "components/devices/DeviceSummary";
import { DeviceSummaryManaged } from "components/devices/DeviceSummaryManaged";
import { useCallback } from "react";
import Component, { Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceSummaryContext } = props;
  const { deviceId } = deviceSummaryContext;

  const data = useDeviceSummary<Data>(deviceSummaryContext);

  const onR = useCallback((): void => {
    deviceClassPostEmpty(deviceId, "/r");
  }, [deviceId]);
  const onS = useCallback((): void => {
    deviceClassPostEmpty(deviceId, "/s");
  }, [deviceId]);
  const onT = useCallback((): void => {
    deviceClassPostEmpty(deviceId, "/t");
  }, [deviceId]);

  return <Component data={data} onR={onR} onS={onS} onT={onT} />;
};
export default ManagedComponent;
