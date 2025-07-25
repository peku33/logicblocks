import { type Temperature } from "./Temperature";

// TODO: this should be inside devices/houseblocks folder

export interface Ds18x20State {
  sensor_type: "Empty" | "Invalid" | "S" | "B";
  reset_count: number;
  temperature: Temperature | null;
}
