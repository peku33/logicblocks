import { useDeviceSummary } from "components/devices/DeviceSummary";
import { DeviceSummaryManaged } from "components/devices/DeviceSummaryManaged";
import Component, { Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceSummaryContext } = props;

  const data = useDeviceSummary<Data>(deviceSummaryContext);

  return <Component data={data} />;
};
export default ManagedComponent;
