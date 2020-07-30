import SSEAggregatedClient from "lib/SSEAggregatedClient";
import { useState, useEffect } from "react";
import useAsyncEffect from "use-async-effect";
import { getJson, postEmpty } from "lib/Api";

const sseAggregatedStream = new SSEAggregatedClient("/device_runner/devices/events");

export async function postDeviceEmpty(deviceId: number, endpoint: string): Promise<void> {
  return await postEmpty(`/device_runner/devices/${deviceId}/device${endpoint}`);
}

export function useDeviceState<T>(
  deviceId: number,
): {
  deviceState: T | undefined;
  invalidateDeviceState: () => void;
  version: number;
} {
  const version = useDeviceStateVersion(deviceId);
  const [deviceState, setDeviceState] = useState<T>();

  useAsyncEffect(
    async (isMounted) => {
      const deviceState = await getJson<T>(`/device_runner/devices/${deviceId}/device`);
      if (!isMounted) return;
      setDeviceState(deviceState);
    },
    () => {
      setDeviceState(undefined);
    },
    [deviceId, version],
  );

  const invalidateDeviceState = (): void => {
    setDeviceState(undefined);
  };

  return { deviceState, invalidateDeviceState, version };
}

export function useDeviceStateVersion(deviceId: number): number {
  const [version, setVersion] = useState(0);
  useEffect(() => {
    const token = sseAggregatedStream.watchAdd([deviceId, "device"], () => {
      setVersion((version) => version + 1);
    });
    return (): void => {
      sseAggregatedStream.watchRemove(token);
    };
  }, [deviceId]);
  return version;
}
