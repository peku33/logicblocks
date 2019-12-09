import { urlBuild } from "../../services/Api";

export default class DeviceContext {
  public constructor(
    public readonly deviceId: number,
  ) { }

  public urlBuild(endpoint: string): string {
    return urlBuild(`/device_pool/${this.deviceId}${endpoint}`);
  }
}
