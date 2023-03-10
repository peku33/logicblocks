import { deviceClassPostEmpty, deviceClassPostJsonEmpty } from "components/devices/Device";
import { DeviceSummaryManaged } from "components/devices/DeviceSummaryManaged";
import { useDeviceSummary } from "components/devices/DeviceSummaryService";
import { useCallback } from "react";
import Component, { Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummary<Data>(deviceId);

  const onModeSet = useCallback(
    (value: boolean | null): void => {
      deviceClassPostJsonEmpty(deviceId, "/mode/set", value);
    },
    [deviceId],
  );
  const onModeCycle = useCallback((): void => {
    deviceClassPostEmpty(deviceId, "/mode/cycle");
  }, [deviceId]);

  return <Component data={data} onModeSet={onModeSet} onModeCycle={onModeCycle} />;
};
export default ManagedComponent;
