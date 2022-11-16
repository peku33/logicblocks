export function clamp(value: number, min: number, max: number): number {
  if (value < min) {
    value = min;
  }
  if (value > max) {
    value = max;
  }
  return value;
}

export function formatSI(value: number, fractionDigits: number, unit: string | undefined): string {
  if (!Number.isFinite(value)) {
    return `${value.toString()}${unit !== undefined ? ` ${unit}` : ``}`;
  }

  const prefixes = ["p", "n", "µ", "m", "", "k", "M", "G", "T"];
  const prefixesShift = 4;

  const exponent = clamp(
    Math.floor(Math.log10(Math.abs(value !== 0 ? value : 1)) / 3),
    -prefixesShift,
    prefixes.length - prefixesShift - 1,
  );
  const prefix = prefixes[exponent + prefixesShift];

  const exponentValue = value * Math.pow(10, -1.0 * exponent * 3);
  const exponentValueRoundMul = Math.pow(10, fractionDigits);
  const exponentValueFormatted = (Math.trunc(exponentValue * exponentValueRoundMul) / exponentValueRoundMul).toFixed(
    fractionDigits,
  );

  return `${exponentValueFormatted}${prefix}${unit !== undefined ? unit : ""}`;
}
