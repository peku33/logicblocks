import React from "react";
import DeviceContext from "./DeviceContext";

const Unknown: React.FC<{
  deviceContext: DeviceContext,
}> = () => {
  return (<span>Unknown</span>);
};

export default Unknown;
