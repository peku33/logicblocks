import { deviceClassPostEmpty, deviceClassPostJsonEmpty } from "components/devices/Device";
import { useDeviceSummary } from "components/devices/DeviceSummary";
import { DeviceSummaryManaged } from "components/devices/DeviceSummaryManaged";
import { useCallback } from "react";
import Component, { Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceSummaryContext } = props;
  const { deviceId } = deviceSummaryContext;

  const data = useDeviceSummary<Data>(deviceSummaryContext);

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
