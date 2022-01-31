import Colors from "components/common/Colors";
import MediaQueries from "components/common/MediaQueries";
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
  display: grid;
  grid-template-columns: 1fr;
  grid-gap: 0.25rem;
  align-items: center;

  @media ${MediaQueries.COMPUTER_AT_LEAST} {
    grid-template-columns: 1fr 2fr;
    grid-gap: 0.5rem;
  }
`;

const Details = styled.div``;
const DetailsName = styled.h2`
  font-size: 1.25rem;
  font-weight: bold;
  word-break: break-all;
`;
const DetailsDetails = styled.h4`
  font-size: 1.125rem;
  font-weight: 600;
  word-break: break-all;
`;

const ComponentWrapper = styled.div`
  @media ${MediaQueries.MOBILE_ONLY} {
    padding-top: 0.25rem;
    border-top: solid 1px ${Colors.GREY_LIGHTEST};
  }
  @media ${MediaQueries.COMPUTER_ONLY} {
    padding-left: 0.5rem;
    border-left: solid 1px ${Colors.GREY_LIGHTEST};
  }
`;
