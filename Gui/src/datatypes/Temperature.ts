import { formatSI } from "util/Number";

type Temperature = number; // as kelvin
export default Temperature;

export function temperatureToCelsius(temperature: Temperature): number {
  return temperature - 273.15;
}
export function temperatureToFahrenheit(temperature: Temperature): number {
  return temperature * (9.0 / 5.0) - 459.67;
}

export function formatTemperatureCelsius(temperature: Temperature, decimalPoints: number): string {
  return formatSI(temperatureToCelsius(temperature), decimalPoints, "°C");
}
export function formatTemperatureCelsiusOrUnknown(
  temperature: Temperature | null | undefined,
  decimalPoints: number,
): string {
  if (temperature == null) {
    return "?";
  }

  return formatTemperatureCelsius(temperature, decimalPoints);
}

export function formatTemperatureFahrenheit(temperature: Temperature, decimalPoints: number): string {
  return formatSI(temperatureToFahrenheit(temperature), decimalPoints, "°F");
}
export function formatTemperatureFahrenheitOrUnknown(
  temperature: Temperature | null | undefined,
  decimalPoints: number,
): string {
  if (temperature == null) {
    return "?";
  }

  return formatTemperatureFahrenheit(temperature, decimalPoints);
}
