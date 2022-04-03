import { Chip, ChipType } from "components/common/Chips";
import styled from "styled-components";

export type DeviceState = "Error" | "Initializing" | "Running";

export interface Data {
  device_state: DeviceState;
}

const Component: React.VFC<{
  data: Data | undefined;
}> = (props) => {
  const { data: state } = props;

  return (
    <Wrapper>
      <Chip type={deviceStateToChipState(state?.device_state)} enabled={true}>
        {state !== undefined ? state.device_state : "Unknown"}
      </Chip>
    </Wrapper>
  );
};
export default Component;

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
