import { DeviceSummaryManaged } from "@/components/devices/DeviceSummaryManaged";
import { useDeviceSummary } from "@/components/devices/DeviceSummaryService";
import Component, { Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummary<Data>(deviceId);

  return <Component data={data} />;
};
export default ManagedComponent;
