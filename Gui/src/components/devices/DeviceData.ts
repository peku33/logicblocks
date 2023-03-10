import { getJson } from "lib/Api";
import { DeviceId, deviceEndpointBuild } from "./Device";

export interface DeviceData {
  name: string;
  class: string;
}

export async function fetchDeviceData(deviceId: DeviceId): Promise<DeviceData> {
  return await getJson<DeviceData>(deviceEndpointBuild(deviceId, ""));
}
