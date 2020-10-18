import React from "react";
import { Button, Dimmer, Loader } from "semantic-ui-react";
import { devicePostEmpty, useDeviceSummary } from "services/LogicDevicesRunner";

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
    <Dimmer.Dimmable dimmed={deviceSummary === undefined}>
      <Dimmer active={deviceSummary === undefined} inverted>
        <Loader />
      </Dimmer>
      <Button.Group>
        <Button color={deviceSummary && deviceSummary.value ? "blue" : undefined} onClick={(): void => doS()}>
          SET (On)
        </Button>
        <Button onClick={(): void => doT()}>TOGGLE (Flip)</Button>
        <Button color={deviceSummary && !deviceSummary.value ? "blue" : undefined} onClick={(): void => doR()}>
          RESET (Off)
        </Button>
      </Button.Group>
    </Dimmer.Dimmable>
  );
};

export default Summary;
