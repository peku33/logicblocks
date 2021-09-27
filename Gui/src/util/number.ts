export function clamp(value: number, min: number, max: number): number {
  if (value < min) {
    value = min;
  }
  if (value > max) {
    value = max;
  }
  return value;
}
