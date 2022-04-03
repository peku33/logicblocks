import { ComponentManagedBase } from "components/devices/SummaryManaged";
import { useCallback } from "react";
import { devicePostJsonEmpty } from "services/LogicDevicesRunner";
import Component from "./Summary";

const ComponentManaged: ComponentManagedBase = (props) => {
  const { deviceId } = props;

  const onPush = useCallback(
    (value: boolean): void => {
      devicePostJsonEmpty(deviceId, "", value);
    },
    [deviceId],
  );

  return <Component onPush={onPush} />;
};
export default ComponentManaged;
