import Colors from "components/common/Colors";
import { getJson } from "lib/Api";
import React, { useState } from "react";
import { urlBuild } from "services/LogicDevicesRunner";
import styled from "styled-components";
import useAsyncEffect from "use-async-effect";
import DeviceSummary from "./DeviceSummary";

const DevicesSummary: React.FC = () => {
  const deviceIds = useDeviceIds();

  if (deviceIds === undefined) {
    return null; // TODO
  }

  return (
    <DevicesList>
      {deviceIds.map((deviceId) => (
        <DevicesListItem key={deviceId}>
          <DeviceSummary deviceId={deviceId} />
        </DevicesListItem>
      ))}
    </DevicesList>
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

const DevicesList = styled.div``;
const DevicesListItem = styled.div`
  margin: 0.25rem;
  padding: 0.25rem;
  border-bottom: solid 1px ${Colors.GREY_LIGHTEST};
  &:last-child {
    border-bottom: none;
  }
`;
