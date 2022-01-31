import SummaryManagedWrapperList from "components/devices/SummaryManagedWrapperList";
import { getJson } from "lib/Api";
import React, { useState } from "react";
import { Route, Routes } from "react-router-dom";
import { endpointBuild } from "services/LogicDevicesRunner";
import useAsyncEffect from "use-async-effect";
import Error404 from "./Error404";

const DevicesSummary: React.VFC = () => {
  return (
    <Routes>
      <Route path="" element={<DevicesSummaryListRoute />}></Route>
      <Route path="*" element={<Error404 />} />
    </Routes>
  );
};
export default DevicesSummary;

const DevicesSummaryListRoute: React.VFC = () => {
  const deviceIds = useDeviceIds();

  if (deviceIds === undefined) {
    return null; // TODO
  }

  return <SummaryManagedWrapperList deviceIds={deviceIds} />;
};

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
