import { deviceClassPostEmpty } from "components/devices/Device";
import { DeviceSummaryManaged } from "components/devices/DeviceSummaryManaged";
import { useCallback } from "react";
import Component from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceSummaryContext } = props;
  const { deviceId } = deviceSummaryContext;

  const onSignal = useCallback((): void => {
    deviceClassPostEmpty(deviceId, "");
  }, [deviceId]);

  return <Component onSignal={onSignal} />;
};
export default ManagedComponent;
