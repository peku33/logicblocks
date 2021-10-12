import { SummaryManagedBase } from "components/devices/SummaryManaged";
import { useCallback, useRef } from "react";
import { devicePostJsonEmpty, useDeviceSummary } from "services/LogicDevicesRunner";
import Summary from "./Summary";

const VALUE_TIMEOUT_SECS = 5;
const VALUE_TIMEOUT_SKEW_SECS = 1;

const SummaryManaged: SummaryManagedBase = (props) => {
  const { deviceId } = props;

  const beatTimer = useRef<NodeJS.Timer | null>(null);

  const deviceSummary = useDeviceSummary<boolean>(deviceId);
  const onValueChanged = useCallback(
    (value: boolean): void => {
      // send value to server
      devicePostJsonEmpty(deviceId, "", value);

      // keep refreshing true value
      if (value) {
        if (beatTimer.current === null) {
          beatTimer.current = setInterval(() => {
            devicePostJsonEmpty(deviceId, "", true);
          }, (VALUE_TIMEOUT_SECS - VALUE_TIMEOUT_SKEW_SECS) * 1000);
        }
      } else {
        if (beatTimer.current !== null) {
          clearInterval(beatTimer.current);
          beatTimer.current = null;
        }
      }
    },
    [deviceId],
  );

  return <Summary value={deviceSummary} onValueChanged={onValueChanged} />;
};
export default SummaryManaged;