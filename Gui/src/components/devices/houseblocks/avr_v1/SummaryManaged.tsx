import { useDeviceSummary } from "components/devices/DeviceSummary";
import { DeviceSummaryManaged } from "components/devices/DeviceSummaryManaged";
import { Data } from "./Summary";

export function makeAvrV1SummaryManaged<D extends object>(
  Component: React.ComponentType<{ data: Data<D> | undefined }>,
): DeviceSummaryManaged {
  const ManagedComponent: DeviceSummaryManaged = (props) => {
    const { deviceSummaryContext } = props;

    const data = useDeviceSummary<Data<D>>(deviceSummaryContext);

    return <Component data={data} />;
  };
  return ManagedComponent;
}
