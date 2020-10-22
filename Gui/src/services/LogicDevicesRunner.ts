import { getJson, postEmpty } from "lib/Api";
import { useObservableState } from "lib/ObservableState";
import Client from "lib/SSEAggregatedStream";

export function urlBuild(endpoint: string): string {
  return `/devices_runner${endpoint}`;
}

const devicesSummaryEvents = new Client(urlBuild("/devices/gui-summary-events"));

export function useDeviceSummary<S>(deviceId: number): S | undefined {
  return useObservableState(urlBuild(`/devices/${deviceId}/gui-summary`), devicesSummaryEvents, [deviceId]);
}

// Device web handler
export function deviceUrlBuild(deviceId: number, endpoint: string): string {
  return urlBuild(`/devices/${deviceId}/device${endpoint}`);
}
export async function deviceGetJson<T>(deviceId: number, endpoint: string): Promise<T> {
  return await getJson(deviceUrlBuild(deviceId, endpoint));
}
export async function devicePostEmpty(deviceId: number, endpoint: string): Promise<void> {
  return await postEmpty(deviceUrlBuild(deviceId, endpoint));
}
