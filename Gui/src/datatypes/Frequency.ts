import { formatSI } from "@/util/Number";

export type Frequency = number; // as hertz

export function formatFrequencyHertz(frequency: Frequency, decimalPoints: number): string {
  return formatSI(frequency, decimalPoints, "Hz");
}
export function formatFrequencyHertzOrUnknown(frequency: Frequency | null | undefined, decimalPoints: number): string {
  if (frequency == null) {
    return "?";
  }

  return formatFrequencyHertz(frequency, decimalPoints);
}

export function formatFrequencyRpm(frequency: Frequency, decimalPoints: number): string {
  return formatSI(frequency * 60.0, decimalPoints, "RPM");
}
export function formatFrequencyRpmOrUnknown(frequency: Frequency | null | undefined, decimalPoints: number): string {
  if (frequency == null) {
    return "?";
  }

  return formatFrequencyRpm(frequency, decimalPoints);
}
