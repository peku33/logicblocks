import React from "react";
import { Button } from "semantic-ui-react";
import { devicePostEmpty } from "services/LogicDevicesRunner";

const Summary: React.FC<{
  deviceId: number;
  deviceClass: string;
}> = (props) => {
  const { deviceId } = props;

  const signal = (): void => {
    devicePostEmpty(deviceId, "");
  };

  return <Button onClick={(): void => signal()}>Signal</Button>;
};

export default Summary;
