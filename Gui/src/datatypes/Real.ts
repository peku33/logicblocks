export function formatReal(real: number, decimalPoints: number | undefined): string {
  return decimalPoints !== undefined ? real.toFixed(decimalPoints) : real.toString();
}
export function formatRealOrUnknown(real: number | null | undefined, decimalPoints: number | undefined): string {
  if (real == null) {
    return "?";
  }

  return formatReal(real, decimalPoints);
}
