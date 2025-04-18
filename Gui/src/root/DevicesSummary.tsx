import Loader from "@/components/common/Loader";
import { DeviceId, endpointBuild } from "@/components/devices/Device";
import DeviceSummaryManagedWrapperManagedList from "@/components/devices/DeviceSummaryManagedWrapperManagedList";
import { getJson } from "@/lib/Api";
import { useEffect, useState } from "react";
import { Route, Routes } from "react-router";
import Error404 from "./Error404";

const DevicesSummary: React.FC = () => {
  return (
    <Routes>
      <Route path="" element={<DevicesSummaryListRoute />}></Route>
      <Route path="*" element={<Error404 />} />
    </Routes>
  );
};
export default DevicesSummary;

const DevicesSummaryListRoute: React.FC = () => {
  const deviceIds = useDeviceIds();

  if (deviceIds === undefined) {
    return <Loader sizeRem={4} />;
  }

  return <DeviceSummaryManagedWrapperManagedList deviceIds={deviceIds} />;
};

function useDeviceIds(): DeviceId[] | undefined {
  const [deviceIds, setDeviceIds] = useState<DeviceId[]>();

  useEffect(() => {
    (async () => {
      const deviceIds = await getJson<DeviceId[]>(endpointBuild("/devices/list"));
      const deviceIdsSorted = deviceIds.sort((a, b) => a - b);
      setDeviceIds(deviceIdsSorted);
    })().catch((reason: unknown) => {
      console.error(reason);
    });
  }, []);

  return deviceIds;
}
