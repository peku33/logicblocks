import deepEqual from "deep-equal";
import { useEffect, useMemo, useReducer } from "react";
import { useMemoCompare } from "util/MemoCompare";
import { urlBuild } from "./Api";

export type Topic = number | string;
export type TopicPath = Topic[];

function topicPathCompare(a: TopicPath, b: TopicPath): boolean {
  return deepEqual(a, b);
}

export function useVersions(endpoint: string, topicPaths_: TopicPath[]): number[] {
  const topicPaths = useMemoCompare(topicPaths_, deepEqual);

  const topicPathsLut = useMemo(() => {
    return topicPathsLutBuild(topicPaths);
  }, [topicPaths]);

  const [versions, updateTopicPaths] = useReducer(
    (versions: number[], topicPaths: TopicPath[]): number[] => {
      const indexes = topicPathsLutQuery(topicPathsLut, topicPaths);
      // bump version for each changed index
      return versions.map((version, index) => version + (indexes.has(index) ? 1 : 0));
    },
    undefined,
    () => {
      return Array(topicPaths.length).fill(0);
    },
  );

  useEffect(() => {
    const topicPathsUnique = topicPathsLutUniquePaths(topicPathsLut);
    if (!topicPathsUnique) {
      return;
    }

    const url = urlBuild(endpoint);
    const searchParams = new URLSearchParams({
      filter: topicPathsUnique.map((topicPath) => topicPath.join("-")).join(","),
    });

    const eventSource = new EventSource(`${url}?${searchParams.toString()}`);
    eventSource.addEventListener("error", (event) => {
      console.error("SSETopic", "useVersions", endpoint, event);
    });
    // eventSource.addEventListener('open', (event) => {});
    eventSource.addEventListener("message", (event) => {
      const topicPath: TopicPath = JSON.parse(event.data);
      updateTopicPaths([topicPath]);
    });

    return () => {
      eventSource.close();
    };
  }, [endpoint, topicPathsLut]);

  return versions;
}

type TopicPathsLutHash = string;
type TopicPathsLut = Map<TopicPathsLutHash, Array<[TopicPath, Set<number>]>>;
function topicPathsLutHash(topicPath: TopicPath): string {
  return topicPath.join("/");
}
function topicPathsLutBuild(topicPaths: TopicPath[]): TopicPathsLut {
  const lut = new Map<string, Array<[TopicPath, Set<number>]>>();
  topicPaths.forEach((topicPath, index) => {
    const key = topicPathsLutHash(topicPath);

    // all keys with same hash
    let entries = lut.get(key);
    if (entries === undefined) {
      entries = [];
      lut.set(key, entries);
    }

    // add to set, using deep compare
    let inserted = false;
    entries.forEach(([topicPath_, indexes]) => {
      if (!topicPathCompare(topicPath_, topicPath)) {
        return;
      }

      indexes.add(index);
      inserted = true;
    });
    if (!inserted) {
      entries.push([topicPath, new Set([index])]);
    }
  });
  return lut;
}
function topicPathsLutQuery(lut: TopicPathsLut, topicPaths: TopicPath[]): Set<number> {
  const indexes = new Set<number>();

  topicPaths.forEach((topicPath) => {
    const key = topicPathsLutHash(topicPath);

    const entries = lut.get(key);
    if (entries === undefined) {
      return;
    }

    entries.forEach(([topicPath_, indexes_]) => {
      if (!topicPathCompare(topicPath_, topicPath)) {
        return;
      }

      indexes_.forEach((index) => {
        indexes.add(index);
      });
    });
  });

  return indexes;
}
function topicPathsLutUniquePaths(lut: TopicPathsLut): TopicPath[] {
  const topicPaths: TopicPath[] = [];
  lut.forEach((entry) => {
    const entryTopicPaths: TopicPath[] = [];

    entry.forEach(([topicPath, _indexes]) => {
      const contains = entryTopicPaths.some((topicPath_) => topicPathCompare(topicPath, topicPath_));
      if (contains) {
        return;
      }

      entryTopicPaths.push(topicPath);
    });

    entryTopicPaths.forEach((entryTopicPath) => {
      topicPaths.push(entryTopicPath);
    });
  });
  return topicPaths;
}
