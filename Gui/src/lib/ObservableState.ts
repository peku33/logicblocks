import { useRef, useState } from "react";
import useAsyncEffect from "use-async-effect";
import { getJson } from "./Api";
import Client, { Path, WatchToken } from "./SSEAggregatedStream";

export function useObservableState<S>(
  getJsonUrl: string,

  sseAggregatedStreamClient: Client,
  sseAggregatedStreamPath: Path,
): S | undefined {
  const [state, setState] = useState<S>();
  const token = useRef<WatchToken | null>(null);

  useAsyncEffect(
    (isMounted) => {
      // register data reload on notification and make initial reload
      token.current = sseAggregatedStreamClient.watchAdd(
        sseAggregatedStreamPath,
        async () => {
          const state = await getJson<S>(getJsonUrl);
          if (!isMounted()) return;
          setState(state);
        },
        true,
      );
    },
    () => {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      sseAggregatedStreamClient.watchRemove(token.current!);
    },
    [getJsonUrl, sseAggregatedStreamClient, ...sseAggregatedStreamPath],
  );

  return state;
}
