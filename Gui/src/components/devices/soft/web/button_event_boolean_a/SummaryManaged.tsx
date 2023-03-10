import { deviceClassPostJsonEmpty } from "components/devices/Device";
import { DeviceSummaryManaged } from "components/devices/DeviceSummaryManaged";
import { useDeviceSummary } from "components/devices/DeviceSummaryService";
import { useCallback } from "react";
import Component, { Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummary<Data>(deviceId);

  const onPush = useCallback(
    (value: boolean): void => {
      deviceClassPostJsonEmpty(deviceId, "", value);
    },
    [deviceId],
  );

  return <Component data={data} onPush={onPush} />;
};
export default ManagedComponent;
