import { DeviceSummaryManaged } from "components/devices/DeviceSummaryManaged";
import { useDeviceSummary } from "components/devices/DeviceSummaryService";
import { Data } from "./Summary";

export function makeAvrV1SummaryManaged<D extends object>(
  Component: React.ComponentType<{ data: Data<D> | undefined }>,
): DeviceSummaryManaged {
  const ManagedComponent: DeviceSummaryManaged = (props) => {
    const { deviceId } = props;

    const data = useDeviceSummary<Data<D>>(deviceId);

    return <Component data={data} />;
  };
  return ManagedComponent;
}
