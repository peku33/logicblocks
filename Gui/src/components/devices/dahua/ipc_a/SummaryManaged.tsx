import { SummaryManagedBase } from "components/devices/SummaryManaged";
import { urlBuild } from "lib/Api";
import { deviceEndpointBuild, useDeviceSummary } from "services/LogicDevicesRunner";
import Summary, { DeviceSummary } from "./Summary";

const SummaryManaged: SummaryManagedBase = (props) => {
  const { deviceId } = props;

  const deviceSummary = useDeviceSummary<DeviceSummary>(deviceId);

  return (
    <Summary deviceSummary={deviceSummary} snapshotBaseUrl={urlBuild(deviceEndpointBuild(deviceId, "/snapshot"))} />
  );
};
export default SummaryManaged;
