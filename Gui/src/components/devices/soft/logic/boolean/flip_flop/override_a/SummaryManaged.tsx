import { ComponentManagedBase } from "components/devices/SummaryManaged";
import { useCallback } from "react";
import { devicePostEmpty, devicePostJsonEmpty, useDeviceSummaryData } from "services/LogicDevicesRunner";
import Component, { Data } from "./Summary";

const ComponentManaged: ComponentManagedBase = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummaryData<Data>(deviceId);

  const onModeSet = useCallback(
    (value: boolean | null): void => {
      devicePostJsonEmpty(deviceId, "/mode/set", value);
    },
    [deviceId],
  );
  const onModeCycle = useCallback((): void => {
    devicePostEmpty(deviceId, "/mode/cycle");
  }, [deviceId]);

  return <Component data={data} onModeSet={onModeSet} onModeCycle={onModeCycle} />;
};
export default ComponentManaged;
