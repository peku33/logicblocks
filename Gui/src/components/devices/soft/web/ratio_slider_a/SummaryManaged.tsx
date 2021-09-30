import { SummaryManagedBase } from "components/devices/SummaryManaged";
import { useCallback } from "react";
import { devicePostJsonEmpty, useDeviceSummary } from "services/LogicDevicesRunner";
import Summary from "./Summary";

interface DeviceSummary {
  value: number | null;
}

const SummaryManaged: SummaryManagedBase = (props) => {
  const { deviceId } = props;

  const deviceSummary = useDeviceSummary<DeviceSummary>(deviceId);
  const onValueChanged = useCallback(
    (value: number | null) => {
      devicePostJsonEmpty(deviceId, "", value);
    },
    [deviceId],
  );

  return (
    <Summary value={deviceSummary !== undefined ? deviceSummary.value : undefined} onValueChanged={onValueChanged} />
  );
};
export default SummaryManaged;
