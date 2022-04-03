import { Line } from "components/common/Line";
import { getJson } from "lib/Api";
import { useState } from "react";
import { endpointBuild } from "services/LogicDevicesRunner";
import styled from "styled-components";
import useAsyncEffect from "use-async-effect";
import { getByClass } from "./SummaryManagedFactory";

const SummaryManagedWrapper: React.VFC<{
  deviceId: number;
}> = (props) => {
  const { deviceId } = props;

  const deviceData = useDeviceData(deviceId);
  if (deviceData === undefined) {
    return null;
  }

  const Component = getByClass(deviceData.class);

  return (
    <Wrapper>
      <Details>
        <DetailsName>{deviceData.name}</DetailsName>
        <DetailsDetails>
          #{deviceId} {deviceData.class}
        </DetailsDetails>
      </Details>
      <Line />
      <ComponentWrapper>
        <Component deviceId={deviceId} deviceClass={deviceData.class} />
      </ComponentWrapper>
    </Wrapper>
  );
};
export default SummaryManagedWrapper;

interface DeviceData {
  name: string;
  class: string;
}

function useDeviceData(deviceId: number): DeviceData | undefined {
  const [deviceData, setDeviceData] = useState<DeviceData>();

  useAsyncEffect(
    async (isMounted) => {
      const deviceData = await getJson<DeviceData>(endpointBuild(`/devices/${deviceId}`));
      if (!isMounted()) return;
      setDeviceData(deviceData);
    },
    () => {
      setDeviceData(undefined);
    },
    [deviceId],
  );

  return deviceData;
}

const Wrapper = styled.div`
  display: flex;
  flex-direction: column;
`;

const Details = styled.div``;
const DetailsName = styled.div`
  font-size: large;
  font-weight: bold;
`;
const DetailsDetails = styled.div`
  font-size: x-small;
  font-size: normal;
`;

const ComponentWrapper = styled.div``;
