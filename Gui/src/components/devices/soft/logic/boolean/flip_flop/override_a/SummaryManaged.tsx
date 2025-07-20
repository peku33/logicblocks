import { deviceClassPostEmpty, deviceClassPostJsonEmpty } from "@/components/devices/Device";
import { type DeviceSummaryManaged } from "@/components/devices/DeviceSummaryManaged";
import { useDeviceSummary } from "@/components/devices/DeviceSummaryService";
import { useCallback } from "react";
import Component, { type Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummary<Data>(deviceId);

  const onModeSet = useCallback(
    async (value: boolean | null) => {
      await deviceClassPostJsonEmpty(deviceId, "/mode/set", value);
    },
    [deviceId],
  );
  const onModeCyclePassThrough = useCallback(async () => {
    await deviceClassPostEmpty(deviceId, "/mode/cycle/pass-through");
  }, [deviceId]);
  const onModeCycleNoPassThrough = useCallback(async () => {
    await deviceClassPostEmpty(deviceId, "/mode/cycle/no-pass-through");
  }, [deviceId]);

  return (
    <Component
      data={data}
      onModeSet={onModeSet}
      onModeCyclePassThrough={onModeCyclePassThrough}
      onModeCycleNoPassThrough={onModeCycleNoPassThrough}
    />
  );
};
export default ManagedComponent;
