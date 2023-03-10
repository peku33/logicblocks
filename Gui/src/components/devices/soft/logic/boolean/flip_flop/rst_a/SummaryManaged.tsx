import { deviceClassPostEmpty } from "components/devices/Device";
import { DeviceSummaryManaged } from "components/devices/DeviceSummaryManaged";
import { useDeviceSummary } from "components/devices/DeviceSummaryService";
import { useCallback } from "react";
import Component, { Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummary<Data>(deviceId);

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
