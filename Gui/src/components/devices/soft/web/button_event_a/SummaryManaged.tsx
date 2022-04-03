import { ComponentManagedBase } from "components/devices/SummaryManaged";
import { useCallback } from "react";
import { devicePostEmpty } from "services/LogicDevicesRunner";
import Component from "./Summary";

const ComponentManaged: ComponentManagedBase = (props) => {
  const { deviceId } = props;

  const onSignal = useCallback((): void => {
    devicePostEmpty(deviceId, "");
  }, [deviceId]);

  return <Component onSignal={onSignal} />;
};
export default ComponentManaged;
