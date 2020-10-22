/* eslint-disable @typescript-eslint/no-explicit-any */

import Colors from "components/common/Colors";
import MediaQueries from "components/common/MediaQueries";
import React from "react";
import { useDeviceSummary } from "services/LogicDevicesRunner";
import styled from "styled-components";
import HardwareRunner, { State as HardwareRunnerState } from "./hardware_runner/Summary";

interface DeviceSummary {
  hardware_runner: HardwareRunnerState;
  device: any;
}

export default function makeAvrV1Summary(
  DeviceComponent: React.FC<{
    state?: any;
  }>,
): React.FC<{
  deviceId: number;
  deviceClass: string;
}> {
  const Summary: React.FC<{
    deviceId: number;
    deviceClass: string;
  }> = (props) => {
    const { deviceId } = props;

    const deviceSummary = useDeviceSummary<DeviceSummary>(deviceId);

    return (
      <Wrapper>
        <HardwareRunnerWrapper>
          <HardwareRunner state={deviceSummary?.hardware_runner} />
        </HardwareRunnerWrapper>
        <DeviceComponentWrapper>
          <DeviceComponent state={deviceSummary?.device} />
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
