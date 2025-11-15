import { formatSI } from "@/util/Number";

export type Current = number; // as amperes

export function formatCurrent(current: Current, decimalPoints: number): string {
  return formatSI(current, decimalPoints, "A");
}
export function formatCurrentOrUnknown(current: Current | null | undefined, decimalPoints: number): string {
  if (current == null) {
    return "?";
  }

  return formatCurrent(current, decimalPoints);
}
