import React from "react";
import DeviceContext from "../../DeviceContext";
import Relay14Common from "./Relay14Common";

const Device0006Relay14OptoA: React.FC<{
  deviceContext: DeviceContext,
}> = (props) => {
  return (<Relay14Common
    deviceContext={props.deviceContext}
    deviceClass="logicblocks/avr_v1/0007_relay14_ssr_a_v2"
  />);
};

export default Device0006Relay14OptoA;
