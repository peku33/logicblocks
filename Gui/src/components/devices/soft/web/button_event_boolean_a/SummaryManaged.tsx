import { SummaryManagedBase } from "components/devices/SummaryManaged";
import { useCallback } from "react";
import { devicePostJsonEmpty } from "services/LogicDevicesRunner";
import Summary from "./Summary";

const SummaryManaged: SummaryManagedBase = (props) => {
  const { deviceId } = props;

  const onPush = useCallback(
    (value: boolean): void => {
      devicePostJsonEmpty(deviceId, "", value);
    },
    [deviceId],
  );

  return <Summary onPush={onPush} />;
};
export default SummaryManaged;
