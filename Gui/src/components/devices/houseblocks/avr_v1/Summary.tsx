/* eslint-disable @typescript-eslint/no-explicit-any */

import React from "react";
import { Dimmer, Grid, Loader } from "semantic-ui-react";
import { useDeviceSummary } from "services/LogicDevicesRunner";
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
      <Dimmer.Dimmable dimmed={deviceSummary === undefined}>
        <Dimmer active={deviceSummary === undefined} inverted>
          <Loader />
        </Dimmer>
        <Grid columns={2} divided verticalAlign="top">
          <Grid.Column mobile={16} computer={4}>
            <HardwareRunner state={deviceSummary?.hardware_runner} />
          </Grid.Column>
          <Grid.Column mobile={16} computer={12}>
            <DeviceComponent state={deviceSummary?.device} />
          </Grid.Column>
        </Grid>
      </Dimmer.Dimmable>
    );
  };

  return Summary;
}
