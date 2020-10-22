import Colors from "components/common/Colors";
import MediaQueries from "components/common/MediaQueries";
import { getSummaryComponent } from "components/devices/Factory";
import { getJson } from "lib/Api";
import React, { useState } from "react";
import { urlBuild } from "services/LogicDevicesRunner";
import styled from "styled-components";
import useAsyncEffect from "use-async-effect";

interface DeviceData {
  name: string;
  class: string;
}

const DeviceSummary: React.FC<{
  deviceId: number;
}> = (props) => {
  const { deviceId } = props;

  const deviceData = useDeviceContext(deviceId);

  if (deviceData === undefined) {
    return null; // TODO
  }

  const Component = getSummaryComponent(deviceData.class);

  return (
    <Wrapper>
      <DeviceDetails>
        <DeviceDetailsName>{deviceData.name}</DeviceDetailsName>
        <DeviceDetailsDetails>
          #{deviceId} {deviceData.class}
        </DeviceDetailsDetails>
      </DeviceDetails>
      <DeviceComponentWrapper>
        <Component deviceId={deviceId} deviceClass={deviceData.class} />
      </DeviceComponentWrapper>
    </Wrapper>
  );
};

export default DeviceSummary;

function useDeviceContext(deviceId: number): DeviceData | undefined {
  const [deviceData, setDeviceData] = useState<DeviceData>();

  useAsyncEffect(
    async (isMounted) => {
      const deviceData = await getJson<DeviceData>(urlBuild(`/devices/${deviceId}`));
      if (!isMounted) return;
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
    grid-gap: 1rem;
  }
`;
const DeviceDetails = styled.div``;
const DeviceDetailsName = styled.h2`
  font-size: 1.25rem;
  font-weight: bold;
  word-break: break-all;
`;
const DeviceDetailsDetails = styled.h4`
  font-size: 1.125rem;
  font-weight: 600;
  word-break: break-all;
`;
const DeviceComponentWrapper = styled.div`
  @media ${MediaQueries.MOBILE_ONLY} {
    padding-top: 0.25rem;
    border-top: solid 1px ${Colors.GREY_LIGHTEST};
  }
  @media ${MediaQueries.COMPUTER_ONLY} {
    padding-left: 1rem;
    border-left: solid 1px ${Colors.GREY_LIGHTEST};
  }
`;
