import DahuaIpc from "./dahua/Ipc";
import DeviceContext from "./DeviceContext";
import LogicblocksAvrV1Device0006Relay14OptoA from "./logicblocks/avr_v1/Device0006Relay14OptoA";
import Unknown from "./Unknown";

export type DeviceComponent = React.FC<{
  deviceContext: DeviceContext,
}>;

export function getComponentClassForDevice(
  deviceClass: string,
): DeviceComponent {
  switch (deviceClass) {
    case "dahua/ipc": return DahuaIpc;
    case "logicblocks/avr_v1/0006_relay14_opto_a": return LogicblocksAvrV1Device0006Relay14OptoA;
    default: return Unknown;
  }
}
