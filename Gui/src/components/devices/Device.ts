import { getJson, postEmpty, postJsonEmpty } from "@/lib/Api";

// from service
export function endpointBuild(endpoint: string): string {
  return `/devices-runner${endpoint}`;
}

export type DeviceId = number;

// from wrapper
export function deviceEndpointBuild(deviceId: DeviceId, endpoint: string): string {
  return endpointBuild(`/devices/${deviceId}${endpoint}`);
}

// from device class
export function deviceClassEndpointBuild(deviceId: DeviceId, endpoint: string): string {
  return deviceEndpointBuild(deviceId, `/device${endpoint}`);
}
export async function deviceClassGetJson<T>(deviceId: DeviceId, endpoint: string): Promise<T> {
  return await getJson(deviceClassEndpointBuild(deviceId, endpoint));
}
export async function deviceClassPostEmpty(deviceId: DeviceId, endpoint: string): Promise<void> {
  await postEmpty(deviceClassEndpointBuild(deviceId, endpoint));
}
// eslint-disable-next-line @typescript-eslint/no-unnecessary-type-parameters
export async function deviceClassPostJsonEmpty<D>(deviceId: DeviceId, endpoint: string, data: D): Promise<void> {
  await postJsonEmpty(deviceClassEndpointBuild(deviceId, endpoint), data);
}
