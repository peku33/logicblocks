import { SummaryManagedBase } from "components/devices/SummaryManaged";
import { useCallback } from "react";
import { devicePostEmpty } from "services/LogicDevicesRunner";
import Summary from "./Summary";

const SummaryManaged: SummaryManagedBase = (props) => {
  const { deviceId } = props;

  const doSignal = useCallback((): void => {
    devicePostEmpty(deviceId, "");
  }, [deviceId]);

  return <Summary onSignal={doSignal} />;
};
export default SummaryManaged;
