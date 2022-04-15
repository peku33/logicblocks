import { ComponentManagedBase } from "components/devices/SummaryManaged";
import { useCallback } from "react";
import { devicePostJsonEmpty, useDeviceSummaryData } from "services/LogicDevicesRunner";
import Component, { Data } from "./Summary";

const ComponentManaged: ComponentManagedBase = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummaryData<Data>(deviceId);

  const onPush = useCallback(
    (value: boolean): void => {
      devicePostJsonEmpty(deviceId, "", value);
    },
    [deviceId],
  );

  return <Component data={data} onPush={onPush} />;
};
export default ComponentManaged;
