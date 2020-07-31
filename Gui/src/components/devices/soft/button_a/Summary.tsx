import React from "react";
import { Button } from "semantic-ui-react";
import { postDeviceEmpty } from "services/LogicRunnerDevices";

const Summary: React.FC<{
  deviceId: number;
  deviceClass: string;
}> = (props) => {
  const { deviceId } = props;

  const signal = (): void => {
    postDeviceEmpty(deviceId, "");
  };

  return <Button onClick={(): void => signal()}>Signal</Button>;
};

export default Summary;
