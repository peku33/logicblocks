import assert from "assert";
import deepEqual from "deep-equal";
import { urlBuild } from "./Api";

export type Topic = number | string;
export class TopicPath {
  public constructor(private readonly topicPath: Topic[]) {
    // TODO: check for forbidden chars in topic
  }

  public equals(other: this): boolean {
    return deepEqual(this.topicPath, other.topicPath);
  }
  public hash(): string {
    return this.topicPath.join("/");
  }

  public size(): number {
    return this.topicPath.length;
  }

  public iter(): Iterable<Topic> {
    return this.topicPath.values();
  }
}
export class TopicPaths {
  private topicPathsMap = new Map<string, TopicPath>();

  public constructor(topicPaths: TopicPath[]) {
    for (const topicPath of topicPaths) {
      this.add(topicPath);
    }
  }

  public equals(other: this): boolean {
    if (this.topicPathsMap.keys() !== other.topicPathsMap.keys()) {
      return false;
    }

    for (const [hash, topicPath] of this.topicPathsMap) {
      // we checked for key equality, so both should have same keys
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      if (!other.topicPathsMap.get(hash)!.equals(topicPath)) {
        return false;
      }
    }

    return true;
  }

  public empty(): boolean {
    return this.topicPathsMap.size === 0;
  }
  public add(topicPath: TopicPath) {
    const hash = topicPath.hash();
    this.topicPathsMap.set(hash, topicPath);
  }
  public iter(): Iterable<TopicPath> {
    return this.topicPathsMap.values();
  }
}

export interface ClientSubscription {
  opened: () => void;
  message: (topicPath: TopicPath) => void;
}
export class ClientSubscriptionToken {
  public constructor(public readonly client: Client) {}
}
export class Client {
  public constructor(public readonly endpoint: string, public readonly topicPaths: TopicPaths) {
    assert(!this.topicPaths.empty());

    this.eventSourceCreate();
  }
  public close() {
    assert(this.subscriptions.size === 0);

    this.eventSourceDestroy();
  }

  // subscriptions
  private subscriptions = new Map<ClientSubscriptionToken, ClientSubscription>();
  public subscriptionAdd(subscription: ClientSubscription): ClientSubscriptionToken {
    const token = new ClientSubscriptionToken(this);
    this.subscriptions.set(token, subscription);
    return token;
  }
  public subscriptionRemove(token: ClientSubscriptionToken) {
    assert(token.client === this);
    this.subscriptions.delete(token);
  }

  // eventSource
  private eventSource: EventSource | undefined = undefined;
  private eventSourceCreate() {
    if (this.eventSource !== undefined) return;

    const url = urlBuild(this.endpoint);
    const searchParams = new URLSearchParams({
      filter: [...this.topicPaths.iter()].map((topicPath) => [...topicPath.iter()].join("-")).join(","),
    });

    this.eventSource = new EventSource(`${url}?${searchParams.toString()}`);
    this.eventSource.addEventListener("error", (event) => {
      this.eventSourceOnError(event);
    });
    this.eventSource.addEventListener("open", (event) => {
      this.eventSourceOnOpen(event);
    });
    this.eventSource.addEventListener("message", (event) => {
      this.eventSourceOnMessage(event);
    });
  }
  private eventSourceDestroy() {
    if (this.eventSource === undefined) return;
    this.eventSource.close();
  }
  private eventSourceOnError(event: Event) {
    console.error("SSETopic", "Client", this.endpoint, this.topicPaths, event);
  }
  private eventSourceOnOpen(event: Event) {
    this.handleOpen();
  }
  private eventSourceOnMessage(event: MessageEvent) {
    const topicPath = JSON.parse(event.data);

    assert(Array.isArray(topicPath));

    for (const topic of topicPath) {
      assert(typeof topic === "string" || typeof topic === "number");
    }

    this.handleMessage(new TopicPath(topicPath));
  }

  // common
  private handleOpen() {
    this.subscriptions.forEach((subscription) => subscription.opened());
  }
  private handleMessage(topicPath: TopicPath) {
    this.subscriptions.forEach((subscription) => subscription.message(topicPath));
  }
}
