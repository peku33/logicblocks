import { getJson } from "lib/Api";
import React, { useState } from "react";
import { List, Loader } from "semantic-ui-react";
import { urlBuild } from "services/LogicDevicesRunner";
import useAsyncEffect from "use-async-effect";
import DeviceSummary from "./DeviceSummary";

const DevicesSummary: React.FC = () => {
  const deviceIds = useDeviceIds();

  if (deviceIds === undefined) {
    return <Loader active />;
  }

  return (
    <List relaxed divided>
      {deviceIds.map((deviceId) => (
        <List.Item key={deviceId}>
          <DeviceSummary deviceId={deviceId} />
        </List.Item>
      ))}
    </List>
  );
};

export default DevicesSummary;

function useDeviceIds(): number[] | undefined {
  const [deviceIds, setDeviceIds] = useState<number[]>();

  useAsyncEffect(
    async (isMounted) => {
      const deviceIds = await getJson<number[]>(urlBuild("/devices/list"));
      const deviceIdsSorted = deviceIds.sort((a, b) => a - b);
      if (!isMounted) return;
      setDeviceIds(deviceIdsSorted);
    },
    () => {
      setDeviceIds(undefined);
    },
    [],
  );

  return deviceIds;
}
