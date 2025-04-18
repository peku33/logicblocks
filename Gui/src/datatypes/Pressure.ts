import { formatSI } from "@/util/Number";

type Pressure = number; // as pascals
export default Pressure;

export function formatPressure(pressure: Pressure, decimalPoints: number) {
  return formatSI(pressure, decimalPoints, "pa");
}
export function formatPressureOrUnknown(pressure: Pressure | null | undefined, decimalPoints: number): string {
  if (pressure == null) {
    return "?";
  }

  return formatPressure(pressure, decimalPoints);
}
