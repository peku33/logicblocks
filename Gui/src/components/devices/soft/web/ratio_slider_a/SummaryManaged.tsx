import { SummaryManagedBase } from "components/devices/SummaryManaged";
import { useCallback } from "react";
import { devicePostJsonEmpty, useDeviceSummary } from "services/LogicDevicesRunner";
import Summary from "./Summary";

const SummaryManaged: SummaryManagedBase = (props) => {
  const { deviceId } = props;

  const deviceSummary = useDeviceSummary<number | null>(deviceId);
  const onValueChanged = useCallback(
    (value: number | null) => {
      devicePostJsonEmpty(deviceId, "", value);
    },
    [deviceId],
  );

  return <Summary value={deviceSummary} onValueChanged={onValueChanged} />;
};
export default SummaryManaged;
