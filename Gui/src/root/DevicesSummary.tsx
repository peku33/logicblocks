import Loader from "components/common/Loader";
import { DeviceId, endpointBuild } from "components/devices/Device";
import DeviceSummaryManagedWrapperManagedList from "components/devices/DeviceSummaryManagedWrapperManagedList";
import { getJson } from "lib/Api";
import { useState } from "react";
import { Route, Routes } from "react-router-dom";
import useAsyncEffect from "use-async-effect";
import Error404 from "./Error404";

const DevicesSummary: React.FC<{}> = () => {
  return (
    <Routes>
      <Route path="" element={<DevicesSummaryListRoute />}></Route>
      <Route path="*" element={<Error404 />} />
    </Routes>
  );
};
export default DevicesSummary;

const DevicesSummaryListRoute: React.FC<{}> = () => {
  const deviceIds = useDeviceIds();

  if (deviceIds === undefined) {
    return <Loader sizeRem={4} />;
  }

  return <DeviceSummaryManagedWrapperManagedList deviceIds={deviceIds} />;
};

function useDeviceIds(): DeviceId[] | undefined {
  const [deviceIds, setDeviceIds] = useState<DeviceId[]>();

  useAsyncEffect(async (isMounted) => {
    const deviceIds = await getJson<DeviceId[]>(endpointBuild("/devices/list"));
    const deviceIdsSorted = deviceIds.sort((a, b) => a - b);
    if (!isMounted()) return;
    setDeviceIds(deviceIdsSorted);
  }, []);

  return deviceIds;
}
