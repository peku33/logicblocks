import React, { useState } from "react";
import { Loader, List } from "semantic-ui-react";
import DeviceSummary from "./DeviceSummary";
import useAsyncEffect from "use-async-effect";
import { getJson } from "lib/Api";

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
      const deviceIds = await getJson<number[]>("/device_runner/devices/list");
      const deviceIdsSorted = deviceIds.sort();
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
