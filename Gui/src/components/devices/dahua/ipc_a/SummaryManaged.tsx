import { ComponentManagedBase } from "components/devices/SummaryManaged";
import { urlBuild } from "lib/Api";
import { deviceEndpointBuild, useDeviceSummaryData } from "services/LogicDevicesRunner";
import Component, { Data } from "./Summary";

const ComponentManaged: ComponentManagedBase = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummaryData<Data>(deviceId);

  return <Component data={data} snapshotBaseUrl={urlBuild(deviceEndpointBuild(deviceId, "/snapshot"))} />;
};
export default ComponentManaged;
