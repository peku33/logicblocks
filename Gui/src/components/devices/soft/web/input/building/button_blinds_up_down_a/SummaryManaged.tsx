import { deviceClassPostJsonEmpty } from "@/components/devices/Device";
import { type DeviceSummaryManaged } from "@/components/devices/DeviceSummaryManaged";
import { useDeviceSummary } from "@/components/devices/DeviceSummaryService";
import { useCallback, useEffect, useRef } from "react";
import Component from "./Summary";

const VALUE_TIMEOUT_SECS = 5;
const VALUE_TIMEOUT_SKEW_SECS = 1;

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  const beatTimer = useRef<NodeJS.Timeout | null>(null);

  const data = useDeviceSummary<boolean | null>(deviceId);

  const onValueChanged = useCallback(
    (value: boolean | null) => {
      // send value to server
      deviceClassPostJsonEmpty(deviceId, "", value).catch((reason: unknown) => {
        console.error(reason);
      });

      // we always want to reset the timer
      // if value is null - we simply don't want it anymore
      // if value is false/true - we have to recreate it, to start counting from now
      if (beatTimer.current !== null) {
        clearInterval(beatTimer.current);
        beatTimer.current = null;
      }
      if (value !== null) {
        beatTimer.current = setInterval(
          () => {
            deviceClassPostJsonEmpty(deviceId, "", value).catch((reason: unknown) => {
              console.error(reason);
            });
          },
          (VALUE_TIMEOUT_SECS - VALUE_TIMEOUT_SKEW_SECS) * 1000,
        );
      }
    },
    [deviceId],
  );

  // disable on component unmount
  useEffect(() => {
    return () => {
      if (beatTimer.current === null) {
        return;
      }

      deviceClassPostJsonEmpty(deviceId, "", null).catch((reason: unknown) => {
        console.error(reason);
      });
      clearInterval(beatTimer.current);
      beatTimer.current = null;
    };
  }, [deviceId]);

  return <Component data={data} onValueChanged={onValueChanged} />;
};
export default ManagedComponent;
