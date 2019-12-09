import DahuaIpc from "./dahua/Ipc";
import DeviceContext from "./DeviceContext";
import Unknown from "./Unknown";

export type DeviceComponent = React.FC<{
  deviceContext: DeviceContext,
}>;

export function getComponentClassForDevice(
  deviceClass: string,
): DeviceComponent {
  switch (deviceClass) {
    case "dahua/ipc": return DahuaIpc;
    default: return Unknown;
  }
}
