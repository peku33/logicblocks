import React from "react";
import DeviceContext from "../../DeviceContext";
import Relay14Common from "./Relay14Common";

const Device0006Relay14OptoA: React.FC<{
  deviceContext: DeviceContext,
}> = (props) => {
  return (<Relay14Common
    deviceContext={props.deviceContext}
    deviceClass="logicblocks/avr_v1/0006_relay14_opto_a_v1"
  />);
};

export default Device0006Relay14OptoA;
