import { deviceClassPostJsonEmpty } from "@/components/devices/Device";
import { type DeviceSummaryManaged } from "@/components/devices/DeviceSummaryManaged";
import { useDeviceSummary } from "@/components/devices/DeviceSummaryService";
import { useCallback } from "react";
import Component, { type Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummary<Data>(deviceId);

  const onPush = useCallback(
    async (value: boolean) => {
      await deviceClassPostJsonEmpty(deviceId, "", value);
    },
    [deviceId],
  );

  return <Component data={data} onPush={onPush} />;
};
export default ManagedComponent;
