import { urlBuild } from "../../services/Api";

export default class DeviceContext {
  public constructor(
    public readonly deviceId: number,
  ) { }

  public endpointBuild(endpoint: string): string {
    return `/device_pool/${this.deviceId}${endpoint}`;
  }
  public urlBuild(endpoint: string): string {
    return urlBuild(this.endpointBuild(endpoint));
  }
}
