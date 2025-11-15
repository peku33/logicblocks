import { formatSI } from "@/util/Number";

export type FlowVolumetric = number; // as m^3/s

export function formatFlowVolumetricCubicMetersPerSecond(
  flow_volumetric: FlowVolumetric,
  decimalPoints: number,
): string {
  return formatSI(flow_volumetric, decimalPoints, "m^3/s");
}
export function formatFlowVolumetricCubicMetersPerSecondOrUnknown(
  flow_volumetric: FlowVolumetric | null | undefined,
  decimalPoints: number,
): string {
  if (flow_volumetric == null) {
    return "?";
  }

  return formatFlowVolumetricCubicMetersPerSecond(flow_volumetric, decimalPoints);
}

export function formatFlowVolumetricLitersPerMinute(flow_volumetric: FlowVolumetric, decimalPoints: number): string {
  return formatSI(flow_volumetric * 60.0 * 1000.0, decimalPoints, "l/min");
}
export function formatFlowVolumetricLitersPerMinuteOrUnknown(
  flow_volumetric: FlowVolumetric | null | undefined,
  decimalPoints: number,
): string {
  if (flow_volumetric == null) {
    return "?";
  }

  return formatFlowVolumetricLitersPerMinute(flow_volumetric, decimalPoints);
}
