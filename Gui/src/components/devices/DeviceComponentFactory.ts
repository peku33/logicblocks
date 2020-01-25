import DahuaIpc from "./dahua/Ipc";
import DeviceContext from "./DeviceContext";
import LogicblocksAvrV1Device0006Relay14OptoAV1 from "./logicblocks/avr_v1/Device0006Relay14OptoAV1";
import LogicblocksAvrV1Device0007Relay14SSRAV2 from "./logicblocks/avr_v1/Device0007Relay14SSRAV2";
import Unknown from "./Unknown";

export type DeviceComponent = React.FC<{
  deviceContext: DeviceContext,
}>;

export function getComponentClassForDevice(
  deviceClass: string,
): DeviceComponent {
  switch (deviceClass) {
    case "dahua/ipc": return DahuaIpc;
    case "logicblocks/avr_v1/0006_relay14_opto_a_v1": return LogicblocksAvrV1Device0006Relay14OptoAV1;
    case "logicblocks/avr_v1/0007_relay14_ssr_a_v2": return LogicblocksAvrV1Device0007Relay14SSRAV2;
    default: return Unknown;
  }
}
