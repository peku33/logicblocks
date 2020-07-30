import React, { useState } from "react";
import { Grid, Header, Loader } from "semantic-ui-react";
import useAsyncEffect from "use-async-effect";
import { getSummaryComponent } from "components/devices/Factory";
import { getJson } from "lib/Api";

interface DeviceData {
  name: string;
  class: string;
}

const DeviceSummary: React.FC<{
  deviceId: number;
}> = (props) => {
  const { deviceId } = props;

  const deviceData = useDeviceContext(deviceId);

  if (deviceData === undefined) {
    return <Loader active />;
  }

  const Component = getSummaryComponent(deviceData.class);

  return (
    <Grid columns={2} divided>
      <Grid.Row>
        <Grid.Column width={4}>
          <Header>
            {deviceData.name}
            <Header.Subheader>
              #{deviceId} {deviceData.class}
            </Header.Subheader>
          </Header>
        </Grid.Column>
        <Grid.Column width={12}>
          <Component deviceId={deviceId} deviceClass={deviceData.class} />
        </Grid.Column>
      </Grid.Row>
    </Grid>
  );
};

export default DeviceSummary;

function useDeviceContext(deviceId: number): DeviceData | undefined {
  const [deviceData, setDeviceData] = useState<DeviceData>();

  useAsyncEffect(
    async (isMounted) => {
      const deviceData = await getJson<DeviceData>(`/device_runner/devices/${deviceId}`);
      if (!isMounted) return;
      setDeviceData(deviceData);
    },
    () => {
      setDeviceData(undefined);
    },
    [deviceId],
  );

  return deviceData;
}
