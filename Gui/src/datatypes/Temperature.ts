type Temperature = number; // as kelvin
export default Temperature;

export function kelvinToCelsius(kelvin: number): number {
  return kelvin - 273.15;
}
export function kelvinToFahrenheit(kelvin: number): number {
  return kelvin * (9.0 / 5.0) - 459.67;
}
