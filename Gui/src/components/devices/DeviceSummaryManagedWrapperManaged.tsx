import Loader from "components/common/Loader";
import { DeviceId } from "./Device";
import { useDeviceData } from "./DeviceDataService";
import DeviceSummaryManagedWrapper from "./DeviceSummaryManagedWrapper";

const DeviceSummaryManagedWrapperManaged: React.FC<{
  deviceId: DeviceId;
}> = (props) => {
  const { deviceId } = props;

  const deviceData = useDeviceData(deviceId);
  if (deviceData === undefined) {
    return <Loader sizeRem={4} />;
  }

  return <DeviceSummaryManagedWrapper deviceId={deviceId} deviceData={deviceData} />;
};
export default DeviceSummaryManagedWrapperManaged;
