export function formatReal(real: number, decimalPoints: number): string {
  return real.toFixed(decimalPoints);
}
export function formatRealOrUndefined(real: number | undefined, decimalPoints: number): string {
  if (real === undefined) {
    return "?";
  }

  return formatReal(real, decimalPoints);
}
