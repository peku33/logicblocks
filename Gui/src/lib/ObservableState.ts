import { useEffect, useState } from "react";
import useAsyncEffect from "use-async-effect";
import { getJson } from "./Api";
import Client, { Path } from "./SSEAggregatedStream";

export function useObservableState<S>(
  getJsonUrl: string,

  sseAggregatedStreamClient: Client,
  sseAggregatedStreamPath: Path,
): S | undefined {
  const [state, setState] = useState<S>();

  const version = useObservableStateVersion(sseAggregatedStreamClient, sseAggregatedStreamPath);

  useAsyncEffect(
    async (isMounted) => {
      const state = await getJson<S>(getJsonUrl);
      if (!isMounted) return;
      setState(state);
    },
    () => {
      setState(undefined);
    },
    [getJsonUrl, sseAggregatedStreamClient, ...sseAggregatedStreamPath, version],
  );

  return state;
}

export function useObservableStateVersion(sseAggregatedStreamClient: Client, sseAggregatedStreamPath: Path): number {
  const [version, setVersion] = useState(0);
  useEffect(() => {
    const token = sseAggregatedStreamClient.watchAdd(sseAggregatedStreamPath, () => {
      setVersion((version) => version + 1);
    });
    return (): void => {
      sseAggregatedStreamClient.watchRemove(token);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sseAggregatedStreamClient, ...sseAggregatedStreamPath]);
  return version;
}
