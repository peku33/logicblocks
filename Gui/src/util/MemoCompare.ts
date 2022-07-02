import { useEffect, useRef } from "react";

export function useMemoCompare<T>(value: T, equals: (a: T, b: T) => boolean): T {
  const ref = useRef(value);
  const equal = equals(ref.current, value);
  useEffect(() => {
    if (!equal) {
      ref.current = value;
    }
  });
  return equal ? ref.current : value;
}
