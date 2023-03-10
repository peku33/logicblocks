import { deviceClassPostJsonEmpty } from "components/devices/Device";
import { DeviceSummaryManaged } from "components/devices/DeviceSummaryManaged";
import { useDeviceSummary } from "components/devices/DeviceSummaryService";
import { useCallback } from "react";
import Component, { Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummary<Data>(deviceId);

  const onValueChanged = useCallback(
    (value: number | null) => {
      deviceClassPostJsonEmpty(deviceId, "", value);
    },
    [deviceId],
  );

  return <Component data={data} onValueChanged={onValueChanged} />;
};
export default ManagedComponent;
