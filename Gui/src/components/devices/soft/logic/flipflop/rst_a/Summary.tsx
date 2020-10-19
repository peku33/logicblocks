import { Button, ButtonGroup } from "components/common/Button";
import React from "react";
import { devicePostEmpty, useDeviceSummary } from "services/LogicDevicesRunner";
import styled from "styled-components";

interface DeviceSummary {
  value: boolean;
}

const Summary: React.FC<{
  deviceId: number;
  deviceClass: string;
}> = (props) => {
  const { deviceId } = props;

  const deviceSummary = useDeviceSummary<DeviceSummary>(deviceId);

  const doR = (): void => {
    devicePostEmpty(deviceId, "/r");
  };
  const doS = (): void => {
    devicePostEmpty(deviceId, "/s");
  };
  const doT = (): void => {
    devicePostEmpty(deviceId, "/t");
  };

  return (
    <Wrapper>
      <ButtonGroup>
        <Button active={deviceSummary && deviceSummary.value} onClick={(): void => doS()}>
          SET (On)
        </Button>
        <Button onClick={(): void => doT()}>TOGGLE (Flip)</Button>
        <Button active={deviceSummary && !deviceSummary.value} onClick={(): void => doR()}>
          RESET (Off)
        </Button>
      </ButtonGroup>
    </Wrapper>
  );
};

export default Summary;

const Wrapper = styled.div``;
