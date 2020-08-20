import React from "react";
import { Button, Dimmer, Loader } from "semantic-ui-react";
import { postDeviceEmpty, useDeviceState } from "services/LogicRunnerDevices";

interface DeviceState {
  value: boolean;
}

const Summary: React.FC<{
  deviceId: number;
  deviceClass: string;
}> = (props) => {
  const { deviceId } = props;

  const { deviceState, invalidateDeviceState } = useDeviceState<DeviceState>(deviceId);

  const doR = (): void => {
    invalidateDeviceState();
    postDeviceEmpty(deviceId, "/r");
  };
  const doS = (): void => {
    invalidateDeviceState();
    postDeviceEmpty(deviceId, "/s");
  };
  const doT = (): void => {
    invalidateDeviceState();
    postDeviceEmpty(deviceId, "/t");
  };

  return (
    <Dimmer.Dimmable dimmed={deviceState === undefined}>
      <Dimmer active={deviceState === undefined} inverted>
        <Loader />
      </Dimmer>
      <Button.Group>
        <Button color={deviceState && deviceState.value ? "blue" : undefined} onClick={(): void => doS()}>
          SET (On)
        </Button>
        <Button onClick={(): void => doT()}>TOGGLE (Flip)</Button>
        <Button color={deviceState && !deviceState.value ? "blue" : undefined} onClick={(): void => doR()}>
          RESET (Off)
        </Button>
      </Button.Group>
    </Dimmer.Dimmable>
  );
};

export default Summary;
