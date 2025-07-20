import { deviceClassPostEmpty } from "@/components/devices/Device";
import { type DeviceSummaryManaged } from "@/components/devices/DeviceSummaryManaged";
import { useCallback } from "react";
import Component from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  const onSignal = useCallback(async () => {
    await deviceClassPostEmpty(deviceId, "");
  }, [deviceId]);

  return <Component onSignal={onSignal} />;
};
export default ManagedComponent;
