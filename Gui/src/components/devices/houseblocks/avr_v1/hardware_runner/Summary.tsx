import { Chip, ChipState } from "components/common/Chips";
import React from "react";
import styled from "styled-components";

type DeviceState = "Error" | "Initializing" | "Running";

export interface State {
  device_state: DeviceState;
}

const Summary: React.FC<{
  state?: State;
}> = (props) => {
  const { state } = props;

  return (
    <Wrapper>
      <Chip chipState={deviceStateToChipState(state?.device_state)}>
        {state !== undefined ? state.device_state : "Unknown"}
      </Chip>
    </Wrapper>
  );
};

export default Summary;

const Wrapper = styled.div`
  display: flex;
`;

function deviceStateToChipState(deviceState: DeviceState | undefined): ChipState {
  switch (deviceState) {
    case undefined:
      return ChipState.UNDEFINED;
    case "Error":
      return ChipState.ERROR;
    case "Initializing":
      return ChipState.WARNING;
    case "Running":
      return ChipState.OK;
  }
}
