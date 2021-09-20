import { SummaryManagedBase } from "components/devices/SummaryManaged";
import { devicePostEmpty } from "services/LogicDevicesRunner";
import Summary from "./Summary";

const SummaryManaged: SummaryManagedBase = (props) => {
  const { deviceId } = props;

  const doSignal = (): void => {
    devicePostEmpty(deviceId, "");
  };

  return <Summary onSignal={doSignal} />;
};
export default SummaryManaged;
