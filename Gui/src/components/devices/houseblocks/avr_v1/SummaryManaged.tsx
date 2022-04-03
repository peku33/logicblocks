import { ComponentManagedBase } from "components/devices/SummaryManaged";
import React from "react";
import { useDeviceSummaryData } from "services/LogicDevicesRunner";
import { Data } from "./Summary";

export function makeAvrV1SummaryManaged<D extends object>(
  SummaryComponent: React.ComponentType<{ data: Data<D> | undefined }>,
): ComponentManagedBase {
  const Summary: ComponentManagedBase = (props) => {
    const { deviceId } = props;

    const data = useDeviceSummaryData<Data<D>>(deviceId);

    return <SummaryComponent data={data} />;
  };

  return Summary;
}
