export function formatDegrees(data: number, decimalPoints: number): string {
  return `${(data * (180.0 / Math.PI)).toFixed(decimalPoints)}°`;
}
export function formatDegreesOrUndefined(data: number | undefined, decimalPoints: number): string {
  if (data === undefined) {
    return "?";
  }

  return formatDegrees(data, decimalPoints);
}
