import { getJson } from "lib/Api";
import { deviceEndpointBuild, DeviceId } from "./Device";

export async function fetchDeviceSummary<T>(deviceId: DeviceId): Promise<T> {
  return await getJson<T>(deviceEndpointBuild(deviceId, "/gui-summary"));
}
