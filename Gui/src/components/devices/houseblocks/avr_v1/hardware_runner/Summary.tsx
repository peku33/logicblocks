import React from "react";
import { Label } from "semantic-ui-react";

type DeviceState = "Error" | "Initializing" | "Running";

export interface State {
  device_state: DeviceState;
}

function deviceStateToColor(deviceState: DeviceState | undefined): "red" | "blue" | "green" {
  switch (deviceState) {
    case "Error":
      return "red";
    case "Initializing":
      return "blue";
    case "Running":
      return "green";
    case undefined:
      return "red";
  }
}

const Summary: React.FC<{
  state?: State;
}> = (props) => {
  const { state } = props;

  return (
    <div>
      <Label color={deviceStateToColor(state?.device_state)}>
        {state !== undefined ? state.device_state : "Unknown"}
      </Label>
    </div>
  );
};

export default Summary;
