import { urlBuild } from "./Api";
import { SubscriptionManager } from "./SubscriptionManager";

export interface SSEListenerEvent {
  id?: string;
  data: string;
}
export class SSEListener extends SubscriptionManager<SSEListenerEvent> {
  public constructor(
    private endpoint: string,
  ) { super(); }

  public toString(): string {
    return `SSEListener(${this.endpoint})`;
  }

  private eventSource?: EventSource;
  protected beforeFirstSubscribe(): void {
    if (!this.eventSource) {
      this.eventSource = new EventSource(urlBuild(this.endpoint));
      this.eventSource.onerror = (error) => {
        console.error(this, error);
      };
      this.eventSource.onmessage = (message) => {
        const id = message.lastEventId;
        const data = message.data;
        this.publish({
          id,
          data,
        });
      };
    }
  }
  protected afterLastUnsubscribe(): void {
    if (this.eventSource) {
      this.eventSource.close();
      this.eventSource = undefined;
    }
  }
}
