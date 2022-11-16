import { formatSI } from "util/Number";

type Resistance = number; // ohms
export default Resistance;

export function resistanceToOhms(resistance: Resistance): number {
  return resistance;
}

export function formatResistance(resistance: Resistance, decimalPoints: number): string {
  return formatSI(resistance, decimalPoints, "Ω");
}
export function formatResistanceOrUnknown(resistance: Resistance | null | undefined, decimalPoints: number): string {
  if (resistance == null) {
    return "?";
  }

  return formatResistance(resistance, decimalPoints);
}
