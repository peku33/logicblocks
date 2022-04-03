import { ComponentManagedBase } from "components/devices/SummaryManaged";
import { useCallback } from "react";
import { devicePostJsonEmpty, useDeviceSummaryData } from "services/LogicDevicesRunner";
import Summary, { Data } from "./Summary";

const ComponentManaged: ComponentManagedBase = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummaryData<Data>(deviceId);

  const onValueChanged = useCallback(
    (value: number | null) => {
      devicePostJsonEmpty(deviceId, "", value);
    },
    [deviceId],
  );

  return <Summary data={data} onValueChanged={onValueChanged} />;
};
export default ComponentManaged;
