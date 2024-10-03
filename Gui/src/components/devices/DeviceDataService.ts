import assert from "assert-ts";
import { useEffect, useState } from "react";
import { DeviceId } from "./Device";
import { DeviceData, fetchDeviceData } from "./DeviceData";

export type AggregatorSubscriptionExecutor = (deviceData: DeviceData | undefined) => void;
export class AggregatorSubscriptionToken {
  public constructor(public readonly device: AggregatorSubscriptionDevice) {}
}
export class AggregatorSubscriptionDevice {
  public constructor(
    public readonly aggregator: Aggregator,
    public readonly deviceId: DeviceId,
  ) {}
  public close() {
    assert(this.subscriptionsEmpty());
  }

  private readonly subscriptionExecutors = new Map<AggregatorSubscriptionToken, AggregatorSubscriptionExecutor>();
  public subscriptionAdd(executor: AggregatorSubscriptionExecutor): AggregatorSubscriptionToken {
    executor(this.deviceData);

    const token = new AggregatorSubscriptionToken(this);

    this.subscriptionExecutors.set(token, executor);

    return token;
  }
  public subscriptionRemove(token: AggregatorSubscriptionToken) {
    assert(token.device === this);

    const executor = this.subscriptionExecutors.get(token);
    assert(executor !== undefined);

    executor(undefined);

    this.subscriptionExecutors.delete(token);
  }
  public subscriptionsEmpty(): boolean {
    return this.subscriptionExecutors.size === 0;
  }
  public subscriptionsPropagate() {
    this.subscriptionExecutors.forEach((executor) => {
      executor(this.deviceData);
    });
  }

  private deviceData: DeviceData | undefined = undefined;
  private deviceDataReloadRunning = false;
  public async deviceDataReload() {
    assert(!this.deviceDataReloadRunning);
    this.deviceDataReloadRunning = true;

    this.deviceData = await fetchDeviceData(this.deviceId);

    this.deviceDataReloadRunning = false;
  }
  public deviceDataReloadRequired(): boolean {
    return this.deviceData === undefined && !this.deviceDataReloadRunning;
  }
}
export class Aggregator {
  private devices = new Map<DeviceId, AggregatorSubscriptionDevice>();
  public deviceAdd(deviceId: DeviceId, executor: AggregatorSubscriptionExecutor): AggregatorSubscriptionToken {
    let device = this.devices.get(deviceId);
    if (device === undefined) {
      device = new AggregatorSubscriptionDevice(this, deviceId);

      this.devices.set(deviceId, device);

      this.deviceDataReloadSchedule();
    }

    const token = device.subscriptionAdd(executor);

    return token;
  }
  public deviceRemove(token: AggregatorSubscriptionToken) {
    assert(token.device.aggregator === this);

    const device = this.devices.get(token.device.deviceId);
    assert(device === token.device);

    device.subscriptionRemove(token);
    if (device.subscriptionsEmpty()) {
      device.close();

      this.devices.delete(token.device.deviceId);
    }
  }

  private deviceDataReloadScheduleImmediate: NodeJS.Immediate | undefined = undefined;
  public deviceDataReloadSchedule() {
    if (this.deviceDataReloadScheduleImmediate !== undefined) return;

    // eslint-disable-next-line @typescript-eslint/no-misused-promises
    this.deviceDataReloadScheduleImmediate = setImmediate(async () => {
      await this.deviceDataReload();
      this.deviceDataReloadScheduleImmediate = undefined;
    });
  }
  public async deviceDataReload() {
    // we use loop, as some subscriptions may be added during await point
    for (;;) {
      // find all devices that require reload
      const subscriptionDevicesReloadRequired: AggregatorSubscriptionDevice[] = [];
      this.devices.forEach((subscriptionDevice) => {
        if (subscriptionDevice.deviceDataReloadRequired()) {
          subscriptionDevicesReloadRequired.push(subscriptionDevice);
        }
      });

      // nothing to reload
      if (subscriptionDevicesReloadRequired.length <= 0) break;

      // first load all to execute callbacks sequentially
      await Promise.all(
        subscriptionDevicesReloadRequired.map((subscriptionDeviceReloadRequired) =>
          subscriptionDeviceReloadRequired.deviceDataReload(),
        ),
      );

      // execute callbacks sequentially
      subscriptionDevicesReloadRequired.forEach((subscriptionDeviceReloadRequired) => {
        subscriptionDeviceReloadRequired.subscriptionsPropagate();
      });
    }
  }
}
export const aggregator = new Aggregator();

export function useDeviceData(deviceId: DeviceId): DeviceData | undefined {
  const [deviceData, setDeviceData] = useState<DeviceData>();

  useEffect(() => {
    const token = aggregator.deviceAdd(deviceId, (deviceData) => {
      setDeviceData(deviceData);
    });
    return () => {
      aggregator.deviceRemove(token);
    };
  }, [deviceId]);

  return deviceData;
}
