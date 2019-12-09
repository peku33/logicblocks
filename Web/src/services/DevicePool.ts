import { get } from "./Api";
import { SSEListener } from "./SSEListener";
import { SubscriptionEventsManager, SubscriptionManagerInterface, SubscriptionStateManager } from "./SubscriptionManager";

export class DevicePoolListener {
  private static instance?: DevicePoolListener;
  public static getInstance(): DevicePoolListener {
    if (!DevicePoolListener.instance) {
      DevicePoolListener.instance = new DevicePoolListener();
    }
    return DevicePoolListener.instance;
  }

  private sseListener = new SSEListener("/device_pool/event_stream");
  private constructor() { }

  public asDeviceListSubscriptionManager(): SubscriptionManagerInterface<void> {
    return {
      subscribe: (token, handler) => this.sseListener.subscribe(token, (event) => {
        if (event.id === undefined && event.data === "") {
          handler();
        }
      }),
      unsubscribe: (token) => this.sseListener.unsubscribe(token),
    };
  }
  public asDeviceStateSubscriptionManager(deviceId: number): SubscriptionManagerInterface<void> {
    const deviceIdString = deviceId.toString();
    return {
      subscribe: (token, handler) => this.sseListener.subscribe(token, (event) => {
        if (event.id === deviceIdString && event.data === "") {
          handler();
        }
      }),
      unsubscribe: (token) => this.sseListener.unsubscribe(token),
    };
  }
  public asDeviceEventsSubscriptionManager(deviceId: number): SubscriptionManagerInterface<string> {
    const deviceIdString = deviceId.toString();
    return {
      subscribe: (token, handler) => this.sseListener.subscribe(token, (event) => {
        if (event.id === deviceIdString && event.data !== "") {
          handler(event.data);
        }
      }),
      unsubscribe: (token) => this.sseListener.unsubscribe(token),
    };
  }
}

export interface DeviceListItem {
  deviceId: number;
  deviceClass: string;
}
export type DeviceListItems = DeviceListItem[];
export class DeviceListManager extends SubscriptionStateManager<DeviceListItems> {
  private static instance?: DeviceListManager;
  public static getInstance(): DeviceListManager {
    if (!DeviceListManager.instance) {
      DeviceListManager.instance = new DeviceListManager();
    }
    return DeviceListManager.instance;
  }

  private constructor() {
    super(DevicePoolListener.getInstance().asDeviceListSubscriptionManager());
  }

  protected load(): Promise<DeviceListItems> {
    return get("/device_pool/");
  }
}

export class DeviceStateManager<DeviceState> extends SubscriptionStateManager<DeviceState> {
  public constructor(
    private readonly deviceId: number,
  ) {
    super(DevicePoolListener.getInstance().asDeviceStateSubscriptionManager(deviceId));
  }

  protected load(): Promise<DeviceState> {
    return get(`/device_pool/${this.deviceId}/`);
  }
}
export class DeviceEventsManager extends SubscriptionEventsManager {
  public constructor(
    private readonly deviceId: number,
  ) {
    super(DevicePoolListener.getInstance().asDeviceEventsSubscriptionManager(deviceId));
  }
}
