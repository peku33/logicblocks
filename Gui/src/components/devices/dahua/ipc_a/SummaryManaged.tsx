import { deviceClassEndpointBuild } from "@/components/devices/Device";
import { DeviceSummaryManaged } from "@/components/devices/DeviceSummaryManaged";
import { useDeviceSummary } from "@/components/devices/DeviceSummaryService";
import { useMemo } from "react";
import Component, { Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummary<Data>(deviceId);

  const snapshotEndpoint = useMemo(() => deviceClassEndpointBuild(deviceId, "/snapshot"), [deviceId]);

  return <Component data={data} snapshotEndpoint={snapshotEndpoint} />;
};
export default ManagedComponent;
