import Colors from "components/common/Colors";
import { getJson } from "lib/Api";
import React, { useState } from "react";
import { Route, Routes } from "react-router-dom";
import { endpointBuild } from "services/LogicDevicesRunner";
import styled from "styled-components";
import useAsyncEffect from "use-async-effect";
import DeviceSummary from "./DeviceSummary";
import Error404 from "./Error404";

const DevicesSummary: React.VFC = () => {
  const deviceIds = useDeviceIds();

  if (deviceIds === undefined) {
    return null; // TODO
  }

  return (
    <Routes>
      <Route
        path=""
        element={
          <>
            <DevicesList>
              {deviceIds.map((deviceId) => (
                <DevicesListItem key={deviceId}>
                  <DeviceSummary deviceId={deviceId} />
                </DevicesListItem>
              ))}
            </DevicesList>
          </>
        }
      ></Route>
      <Route path="*" element={<Error404 />} />
    </Routes>
  );
};
export default DevicesSummary;

function useDeviceIds(): number[] | undefined {
  const [deviceIds, setDeviceIds] = useState<number[]>();

  useAsyncEffect(
    async (isMounted) => {
      const deviceIds = await getJson<number[]>(endpointBuild("/devices/list"));
      const deviceIdsSorted = deviceIds.sort((a, b) => a - b);
      if (!isMounted()) return;
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
  padding: 0.5rem;
  border-bottom: solid 1px ${Colors.GREY_LIGHTEST};
  &:last-child {
    border-bottom: none;
  }
`;
