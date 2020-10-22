import React from "react";
import { devicePostEmpty } from "services/LogicDevicesRunner";
import styled from "styled-components";

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

const Button = styled.div``;
