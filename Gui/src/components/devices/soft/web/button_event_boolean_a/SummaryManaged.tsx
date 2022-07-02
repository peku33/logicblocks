import { deviceClassPostJsonEmpty } from "components/devices/Device";
import { useDeviceSummary } from "components/devices/DeviceSummary";
import { DeviceSummaryManaged } from "components/devices/DeviceSummaryManaged";
import { useCallback } from "react";
import Component, { Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceSummaryContext } = props;
  const { deviceId } = deviceSummaryContext;

  const data = useDeviceSummary<Data>(deviceSummaryContext);

  const onPush = useCallback(
    (value: boolean): void => {
      deviceClassPostJsonEmpty(deviceId, "", value);
    },
    [deviceId],
  );

  return <Component data={data} onPush={onPush} />;
};
export default ManagedComponent;
