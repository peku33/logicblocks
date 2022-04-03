import { getJson, postEmpty, postJsonEmpty } from "lib/Api";
import { useObservableState } from "lib/ObservableState";
import Client from "lib/SSEAggregatedStream";

export function endpointBuild(endpoint: string): string {
  return `/devices-runner${endpoint}`;
}

const devicesSummaryEvents = new Client(endpointBuild("/devices/gui-summary-events"));

export function useDeviceSummaryData<S>(deviceId: number): S | undefined {
  return useObservableState(endpointBuild(`/devices/${deviceId}/gui-summary`), devicesSummaryEvents, [deviceId]);
}

// Device web handler
export function deviceEndpointBuild(deviceId: number, endpoint: string): string {
  return endpointBuild(`/devices/${deviceId}/device${endpoint}`);
}
export async function deviceGetJson<T>(deviceId: number, endpoint: string): Promise<T> {
  return await getJson(deviceEndpointBuild(deviceId, endpoint));
}
export async function devicePostEmpty(deviceId: number, endpoint: string): Promise<void> {
  return await postEmpty(deviceEndpointBuild(deviceId, endpoint));
}
export async function devicePostJsonEmpty<D>(deviceId: number, endpoint: string, data: D): Promise<void> {
  return await postJsonEmpty(deviceEndpointBuild(deviceId, endpoint), data);
}
