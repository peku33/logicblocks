export function formatReal(real: number, decimalPoints: number): string {
  return real.toFixed(decimalPoints);
}
export function formatRealOrUnknown(real: number | null | undefined, decimalPoints: number): string {
  if (real == null) {
    return "?";
  }

  return formatReal(real, decimalPoints);
}
