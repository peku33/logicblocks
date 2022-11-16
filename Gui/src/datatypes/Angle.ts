export function formatDegrees(data: number, decimalPoints: number): string {
  return `${(data * (180.0 / Math.PI)).toFixed(decimalPoints)}°`;
}
export function formatDegreesOrUnknown(data: number | null | undefined, decimalPoints: number): string {
  if (data == null) {
    return "?";
  }

  return formatDegrees(data, decimalPoints);
}
