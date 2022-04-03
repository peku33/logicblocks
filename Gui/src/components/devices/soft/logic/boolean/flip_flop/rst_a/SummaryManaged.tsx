import { ComponentManagedBase } from "components/devices/SummaryManaged";
import { useCallback } from "react";
import { devicePostEmpty, useDeviceSummaryData } from "services/LogicDevicesRunner";
import Component, { Data } from "./Summary";

const ComponentManaged: ComponentManagedBase = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummaryData<Data>(deviceId);

  const onR = useCallback((): void => {
    devicePostEmpty(deviceId, "/r");
  }, [deviceId]);
  const onS = useCallback((): void => {
    devicePostEmpty(deviceId, "/s");
  }, [deviceId]);
  const onT = useCallback((): void => {
    devicePostEmpty(deviceId, "/t");
  }, [deviceId]);

  return <Component data={data} onR={onR} onS={onS} onT={onT} />;
};
export default ComponentManaged;
