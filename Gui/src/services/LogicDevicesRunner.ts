import { getJson, postEmpty } from "lib/Api";
import { useObservableState, useObservableStateVersion } from "lib/ObservableState";
import Client, { Path } from "lib/SSEAggregatedStream";

export function urlBuild(endpoint: string): string {
  return `/devices_runner${endpoint}`;
}
export function deviceUrlBuild(deviceId: number, endpoint: string): string {
  return urlBuild(`/devices/${deviceId}/device${endpoint}`);
}

export async function deviceGetJson<T>(deviceId: number, endpoint: string): Promise<T> {
  return await getJson(deviceUrlBuild(deviceId, endpoint));
}
export async function devicePostEmpty(deviceId: number, endpoint: string): Promise<void> {
  return await postEmpty(deviceUrlBuild(deviceId, endpoint));
}

const sseAggregatedStreamClient = new Client(urlBuild("/events"));

export function devicePathBuild(deviceId: number, path: Path): Path {
  return ["devices", deviceId, "device"].concat(path);
}

export function useDeviceObservableState<S>(
  deviceId: number,
  endpoint: string,
  path: Path,
): {
  state: S | undefined;
  invalidate: () => void;
  version: number;
} {
  return useObservableState(
    deviceUrlBuild(deviceId, endpoint),

    sseAggregatedStreamClient,
    devicePathBuild(deviceId, path),
  );
}
export function useDeviceObservableStateVersion(deviceId: number, path: Path): number {
  return useObservableStateVersion(sseAggregatedStreamClient, devicePathBuild(deviceId, path));
}
