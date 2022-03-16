/* eslint-disable @typescript-eslint/no-explicit-any */

import MediaQueries from "components/common/MediaQueries";
import { SummaryManagedBase } from "components/devices/SummaryManaged";
import { useDeviceSummary } from "services/LogicDevicesRunner";
import styled from "styled-components";
import HardwareRunnerSummary, { State as HardwareRunnerState } from "./hardware_runner/Summary";

export type DeviceSummaryManagedBase = React.VFC<{
  summary: any | undefined;
}>;

interface DeviceSummary {
  hardware_runner: HardwareRunnerState;
  device: any;
}

export function makeAvrV1SummaryManaged(DeviceSummaryManagedBase: DeviceSummaryManagedBase): SummaryManagedBase {
  const Summary: SummaryManagedBase = (props) => {
    const { deviceId } = props;

    const deviceSummary = useDeviceSummary<DeviceSummary>(deviceId);

    return (
      <Wrapper>
        <HardwareRunnerWrapper>
          <HardwareRunnerSummary state={deviceSummary?.hardware_runner} />
        </HardwareRunnerWrapper>
        <DeviceComponentWrapper>
          <DeviceSummaryManagedBase summary={deviceSummary?.device} />
        </DeviceComponentWrapper>
      </Wrapper>
    );
  };

  return Summary;
}

const Wrapper = styled.div`
  display: grid;
  grid-template-columns: 1fr;
  grid-gap: 0.25rem;
  align-items: center;

  @media ${MediaQueries.COMPUTER_ONLY} {
    grid-template-columns: 1fr 4fr;
    grid-gap: 1rem;
  }
`;
const HardwareRunnerWrapper = styled.div``;
const DeviceComponentWrapper = styled.div``;
