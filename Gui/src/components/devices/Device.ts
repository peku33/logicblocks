import { getJson, postEmpty, postJsonEmpty } from "lib/Api";
import { useState } from "react";
import useAsyncEffect from "use-async-effect";

// from service
export function endpointBuild(endpoint: string): string {
  return `/devices-runner${endpoint}`;
}

export type DeviceId = number;

// from wrapper
export function deviceEndpointBuild(deviceId: DeviceId, endpoint: string): string {
  return endpointBuild(`/devices/${deviceId}${endpoint}`);
}

interface DeviceData {
  name: string;
  class: string;
}
export function useDeviceData(deviceId: DeviceId): DeviceData | undefined {
  const [deviceData, setDeviceData] = useState<DeviceData>();

  useAsyncEffect(
    async (isMounted) => {
      const deviceData = await getJson<DeviceData>(deviceEndpointBuild(deviceId, ""));
      if (!isMounted()) return;
      setDeviceData(deviceData);
    },
    () => {
      setDeviceData(undefined);
    },
    [deviceId],
  );

  return deviceData;
}

// from device class
export function deviceClassEndpointBuild(deviceId: DeviceId, endpoint: string): string {
  return deviceEndpointBuild(deviceId, `/device${endpoint}`);
}
export async function deviceClassGetJson<T>(deviceId: DeviceId, endpoint: string): Promise<T> {
  return await getJson(deviceClassEndpointBuild(deviceId, endpoint));
}
export async function deviceClassPostEmpty(deviceId: DeviceId, endpoint: string): Promise<void> {
  return await postEmpty(deviceClassEndpointBuild(deviceId, endpoint));
}
export async function deviceClassPostJsonEmpty<D>(deviceId: DeviceId, endpoint: string, data: D): Promise<void> {
  return await postJsonEmpty(deviceClassEndpointBuild(deviceId, endpoint), data);
}
