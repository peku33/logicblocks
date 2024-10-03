import { deviceClassPostJsonEmpty } from "@/components/devices/Device";
import { DeviceSummaryManaged } from "@/components/devices/DeviceSummaryManaged";
import { useDeviceSummary } from "@/components/devices/DeviceSummaryService";
import { useCallback, useRef } from "react";
import Component from "./Summary";

const VALUE_TIMEOUT_SECS = 5;
const VALUE_TIMEOUT_SKEW_SECS = 1;

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  const beatTimer = useRef<NodeJS.Timeout | null>(null);

  const data = useDeviceSummary<boolean>(deviceId);

  const onValueChanged = useCallback(
    (value: boolean) => {
      // send value to server
      deviceClassPostJsonEmpty(deviceId, "", value).catch((reason: unknown) => {
        console.error(reason);
      });

      // keep refreshing true value
      if (value) {
        if (beatTimer.current === null) {
          beatTimer.current = setInterval(
            () => {
              deviceClassPostJsonEmpty(deviceId, "", true).catch((reason: unknown) => {
                console.error(reason);
              });
            },
            (VALUE_TIMEOUT_SECS - VALUE_TIMEOUT_SKEW_SECS) * 1000,
          );
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

  return <Component data={data} onValueChanged={onValueChanged} />;
};
export default ManagedComponent;
