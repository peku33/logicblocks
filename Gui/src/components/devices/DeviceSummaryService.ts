import assert from "assert";
import * as SSETopic from "lib/SSETopic";
import { useEffect, useState } from "react";
import { DeviceId, endpointBuild } from "./Device";
import { fetchDeviceSummary } from "./DeviceSummary";

const aggregatorEventsClientUrl = endpointBuild("/devices/gui-summary-sse");

export type AggregatorSubscriptionExecutor<T> = (deviceSummary: T | undefined) => void;
export class AggregatorSubscriptionToken {
  public constructor(public readonly device: AggregatorSubscriptionDevice) {}
}
export class AggregatorSubscriptionDevice {
  public constructor(public readonly aggregator: Aggregator, public readonly deviceId: DeviceId) {}
  public close() {
    assert(this.subscriptionsEmpty());
  }

  private readonly subscriptionExecutors = new Map<
    AggregatorSubscriptionToken,
    AggregatorSubscriptionExecutor<unknown>
  >();
  public subscriptionAdd(executor: AggregatorSubscriptionExecutor<unknown>): AggregatorSubscriptionToken {
    executor(this.deviceSummary);

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
    this.subscriptionExecutors.forEach((executor) => executor(this.deviceSummary));
  }

  private deviceSummary: unknown | undefined = undefined;
  private deviceSummaryReloadPending = false;
  private deviceSummaryReloadRunning = false;
  public async deviceSummaryReload() {
    assert(!this.deviceSummaryReloadRunning);

    this.deviceSummaryReloadRunning = true;

    this.deviceSummary = await fetchDeviceSummary(this.deviceId);

    this.deviceSummaryReloadRunning = false;

    this.deviceSummaryReloadPending = false;
  }
  public deviceSummarySetPending() {
    this.deviceSummaryReloadPending = true;
  }
  public deviceSummaryReloadRequired(): boolean {
    return this.deviceSummaryReloadPending && !this.deviceSummaryReloadRunning;
  }
}
export class Aggregator {
  private devices = new Map<DeviceId, AggregatorSubscriptionDevice>();
  public deviceAdd(deviceId: DeviceId, executor: AggregatorSubscriptionExecutor<unknown>): AggregatorSubscriptionToken {
    let device = this.devices.get(deviceId);
    if (device === undefined) {
      // we don't want autoload now, as we will do it on eventsClientReloadSchedule
      device = new AggregatorSubscriptionDevice(this, deviceId);

      this.devices.set(deviceId, device);

      this.eventsClientReloadSchedule();
      this.deviceSummaryReloadSchedule();
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

      this.eventsClientReloadSchedule();
    }
  }

  private eventsClientReloadImmediate: NodeJS.Immediate | undefined = undefined;
  public eventsClientReloadSchedule() {
    if (this.eventsClientReloadImmediate !== undefined) return;

    this.eventsClientReloadImmediate = setImmediate(() => {
      this.eventsClientReload();
      this.eventsClientReloadImmediate = undefined;
    });
  }
  private eventsClient: [SSETopic.Client, SSETopic.ClientSubscriptionToken] | undefined = undefined;
  public eventsClientReload() {
    const sseTopicPaths = new SSETopic.TopicPaths(
      [...this.devices.keys()].map((deviceId) => new SSETopic.TopicPath([deviceId])),
    );

    // check for equality
    if (this.eventsClient === undefined) {
      if (sseTopicPaths.empty()) return;
    } else {
      if (this.eventsClient[0].topicPaths.equals(sseTopicPaths)) return;
    }

    // not equal, must reload client

    // clear current client
    if (this.eventsClient !== undefined) {
      this.eventsClient[0].subscriptionRemove(this.eventsClient[1]);
      this.eventsClient[0].close();
      this.eventsClient = undefined;
    }

    // make new client
    if (!sseTopicPaths.empty()) {
      const eventsClient = new SSETopic.Client(aggregatorEventsClientUrl, sseTopicPaths);
      const token = eventsClient.subscriptionAdd({
        opened: () => this.eventsClientOpened(),
        message: (topicPath) => this.eventsClientMessage(topicPath),
      });
      this.eventsClient = [eventsClient, token];
    }
  }
  private eventsClientOpened() {
    // some events could have been lost while processing the reload
    // reload all devices we have
    for (const device of this.devices.values()) {
      device.deviceSummarySetPending();
    }
    this.deviceSummaryReloadSchedule();
  }
  private eventsClientMessage(topicPath: SSETopic.TopicPath) {
    assert(topicPath.size() === 1);
    const topicPathArray = [...topicPath.iter()];
    const deviceId = topicPathArray[0];
    assert(typeof deviceId === "number");

    const details = this.devices.get(deviceId);
    if (details === undefined) return;
    details.deviceSummarySetPending();
    this.deviceSummaryReloadSchedule();
  }

  private deviceSummaryReloadScheduleImmediate: NodeJS.Immediate | undefined = undefined;
  public deviceSummaryReloadSchedule() {
    if (this.deviceSummaryReloadScheduleImmediate !== undefined) return;

    this.deviceSummaryReloadScheduleImmediate = setImmediate(async () => {
      await this.deviceSummaryReload();
      this.deviceSummaryReloadScheduleImmediate = undefined;
    });
  }
  public async deviceSummaryReload() {
    // we use loop, as some subscriptions may be added during await point
    for (;;) {
      // find all devices that require reload
      const subscriptionDevicesReloadRequired: AggregatorSubscriptionDevice[] = [];
      this.devices.forEach((subscriptionDevice) => {
        if (subscriptionDevice.deviceSummaryReloadRequired()) {
          subscriptionDevicesReloadRequired.push(subscriptionDevice);
        }
      });

      // nothing to reload
      if (subscriptionDevicesReloadRequired.length <= 0) break;

      // first load all to execute callbacks sequentially
      await Promise.all(
        subscriptionDevicesReloadRequired.map((subscriptionDeviceReloadRequired) =>
          subscriptionDeviceReloadRequired.deviceSummaryReload(),
        ),
      );

      // execute callbacks sequentially
      subscriptionDevicesReloadRequired.forEach((subscriptionDeviceReloadRequired) =>
        subscriptionDeviceReloadRequired.subscriptionsPropagate(),
      );
    }
  }
}
export const aggregator = new Aggregator();

export function useDeviceSummary<T>(deviceId: DeviceId): T | undefined {
  const [deviceSummary, setDeviceSummary] = useState<T | undefined>(undefined);

  useEffect(() => {
    const token = aggregator.deviceAdd(deviceId, (deviceSummary) => {
      setDeviceSummary(deviceSummary as T | undefined);
    });
    return () => {
      aggregator.deviceRemove(token);
    };
  }, [deviceId]);

  return deviceSummary;
}
