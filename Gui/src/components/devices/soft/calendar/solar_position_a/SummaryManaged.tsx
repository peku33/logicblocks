import { ComponentManagedBase } from "components/devices/SummaryManaged";
import { useDeviceSummaryData } from "services/LogicDevicesRunner";
import Component, { Data } from "./Summary";

const ComponentManaged: ComponentManagedBase = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummaryData<Data>(deviceId);

  return <Component data={data} />;
};
export default ComponentManaged;
