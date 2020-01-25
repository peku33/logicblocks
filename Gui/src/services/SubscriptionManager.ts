export interface SubscriptionManagerInterface<Event> {
  subscribe(token: any, handler: (event: Event) => void): void;
  unsubscribe(token: any): void;
}

export abstract class SubscriptionManager<Event> {
  private subscriptions: Map<any, (event: Event) => void> = new Map();

  public subscribe(token: any, handler: (event: Event) => void): void {
    if (this.subscriptions.size <= 0) {
      this.beforeFirstSubscribe();
    }
    if (this.subscriptions.has(token)) {
      throw new Error(`Duplicated token: ${token}`);
    }
    this.subscriptions.set(token, handler);
  }
  public unsubscribe(token: any): void {
    if (!this.subscriptions.has(token)) {
      throw new Error(`Missing token: ${token}`);
    }
    this.subscriptions.delete(token);
    if (this.subscriptions.size <= 0) {
      this.afterLastUnsubscribe();
    }
  }

  protected publish(event: Event): void {
    this.subscriptions.forEach((handler) => handler(event));
  }

  protected abstract beforeFirstSubscribe(): void;
  protected abstract afterLastUnsubscribe(): void;
}

export abstract class SubscriptionStateManager<State> {
  private state?: State;
  public constructor(
    private readonly subscriptionManagerInterface: SubscriptionManagerInterface<void>,
  ) { }

  public reactHook(handler: (state: State) => void): () => void {
    this.subscriptionManagerInterface.subscribe(handler, async () => {
      this.state = await this.load();
      handler(this.state);
    });
    if (this.state !== undefined) {
      handler(this.state);
    } else {
      this.load().then((state) => {
        this.state = state;
        handler(this.state);
      });
    }
    return () => this.subscriptionManagerInterface.unsubscribe(handler);
  }
  protected abstract load(): Promise<State>;
}

export class SubscriptionEventsManager {
  public constructor(
    private readonly subscriptionManagerInterface: SubscriptionManagerInterface<string>,
  ) { }

  public reactHook(
    handler: (event: string) => void,
  ): () => void {
    this.subscriptionManagerInterface.subscribe(handler, handler);
    return () => this.subscriptionManagerInterface.unsubscribe(handler);
  }
}
