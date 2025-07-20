import { deviceClassPostEmpty } from "@/components/devices/Device";
import { type DeviceSummaryManaged } from "@/components/devices/DeviceSummaryManaged";
import { useDeviceSummary } from "@/components/devices/DeviceSummaryService";
import { useCallback } from "react";
import Component, { type Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummary<Data>(deviceId);

  const onR = useCallback(async () => {
    await deviceClassPostEmpty(deviceId, "/r");
  }, [deviceId]);
  const onS = useCallback(async () => {
    await deviceClassPostEmpty(deviceId, "/s");
  }, [deviceId]);
  const onT = useCallback(async () => {
    await deviceClassPostEmpty(deviceId, "/t");
  }, [deviceId]);

  return <Component data={data} onR={onR} onS={onS} onT={onT} />;
};
export default ManagedComponent;
