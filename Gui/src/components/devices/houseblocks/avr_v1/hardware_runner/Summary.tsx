import { Chip, ChipType } from "components/common/Chips";
import styled from "styled-components";

type DeviceState = "Error" | "Initializing" | "Running";

export interface State {
  device_state: DeviceState;
}

const Summary: React.VFC<{
  state: State | undefined;
}> = (props) => {
  const { state } = props;

  return (
    <Wrapper>
      <Chip type={deviceStateToChipState(state?.device_state)} enabled={true}>
        {state !== undefined ? state.device_state : "Unknown"}
      </Chip>
    </Wrapper>
  );
};
export default Summary;

const Wrapper = styled.div`
  display: flex;
`;

function deviceStateToChipState(deviceState: DeviceState | undefined): ChipType {
  switch (deviceState) {
    case undefined:
      return ChipType.INFO;
    case "Error":
      return ChipType.ERROR;
    case "Initializing":
      return ChipType.WARNING;
    case "Running":
      return ChipType.OK;
  }
}
