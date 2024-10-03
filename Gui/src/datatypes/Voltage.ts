import { formatSI } from "@/util/Number";

type Voltage = number; // as volts
export default Voltage;

export function formatVoltage(voltage: Voltage, decimalPoints: number): string {
  return formatSI(voltage, decimalPoints, "V");
}
export function formatVoltageOrUnknown(voltage: Voltage | null | undefined, decimalPoints: number): string {
  if (voltage == null) {
    return "?";
  }

  return formatVoltage(voltage, decimalPoints);
}
