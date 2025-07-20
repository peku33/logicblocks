import { getJson } from "@/lib/Api";
import { deviceEndpointBuild, type DeviceId } from "./Device";

export async function fetchDeviceSummary<T>(deviceId: DeviceId): Promise<T> {
  return await getJson<T>(deviceEndpointBuild(deviceId, "/gui-summary"));
}
