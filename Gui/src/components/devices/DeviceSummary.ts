import { getJson } from "lib/Api";
import { useState } from "react";
import useAsyncEffect from "use-async-effect";
import { deviceEndpointBuild, DeviceId } from "./Device";

export interface DeviceSummaryContext {
  deviceId: DeviceId;
  version: number;
}

export function useDeviceSummary<T>(deviceSummaryContext: DeviceSummaryContext): T | undefined {
  const { deviceId, version } = deviceSummaryContext;

  const [deviceSummary, setDeviceSummary] = useState<T | undefined>(undefined);

  useAsyncEffect(
    async (isMounted) => {
      const deviceSummary = await getJson<T>(deviceEndpointBuild(deviceId, "/gui-summary"));
      if (!isMounted()) return;
      setDeviceSummary(deviceSummary);
    },
    [deviceId, version],
  );

  return deviceSummary;
}
