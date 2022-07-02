import { deviceClassEndpointBuild } from "components/devices/Device";
import { useDeviceSummary } from "components/devices/DeviceSummary";
import { DeviceSummaryManaged } from "components/devices/DeviceSummaryManaged";
import { useMemo } from "react";
import Component, { Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceSummaryContext } = props;
  const { deviceId } = deviceSummaryContext;

  const data = useDeviceSummary<Data>(deviceSummaryContext);

  const snapshotEndpoint = useMemo(() => deviceClassEndpointBuild(deviceId, "/snapshot"), [deviceId]);

  return <Component data={data} snapshotEndpoint={snapshotEndpoint} />;
};
export default ManagedComponent;
