import deepEqual from "deep-equal";
import { urlBuild } from "./Api";

export class WatchToken {
  constructor(public readonly token: number) {}
}

export type PathElement = number | string;
export type Path = PathElement[];

interface WatchContext {
  path: Path;
  callback: () => void;
}

export default class Client {
  public constructor(private readonly endpoint: string) {}

  private stream: EventSource | undefined;
  private streamStart(): void {
    if (this.stream !== undefined) {
      throw new Error();
    }

    this.stream = new EventSource(urlBuild(this.endpoint));
    this.stream.onopen = this.streamOnOpen.bind(this);
    this.stream.onmessage = this.streamOnMessage.bind(this);
    this.stream.onerror = this.streamOnError.bind(this);
  }
  private streamStop(): void {
    if (this.stream === undefined) {
      throw new Error();
    }

    this.stream.close();
    this.stream = undefined;
  }

  private streamOnOpen(event: Event): void {
    // TODO: Signal all events after restoring connections
  }
  private streamOnMessage(event: MessageEvent): void {
    const path = JSON.parse(event.data);
    this.onMessage(path);
  }
  private streamOnError(event: Event): void {
    // TODO: Signal all events after restoring connections
    // TODO: Handle event
  }

  private onMessage(path: Path): void {
    this.watches.forEach((context) => {
      if (!deepEqual(path, context.path)) {
        return;
      }

      context.callback();
    });
  }

  private tokenNext = 0;
  private watches: Map<number, WatchContext> = new Map();

  public watchAdd(path: Path, callback: () => void): WatchToken {
    if (this.watches.size === 0) {
      this.streamStart();
    }

    const token = this.tokenNext++;
    const context: WatchContext = {
      path,
      callback,
    };

    this.watches.set(token, context);

    return new WatchToken(token);
  }
  public watchRemove(watchToken: WatchToken): void {
    this.watches.delete(watchToken.token);

    if (this.watches.size === 0) {
      this.streamStop();
    }
  }
}
