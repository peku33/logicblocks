import { Line } from "@/components/common/Line";
import styled from "styled-components";
import { DeviceId } from "./Device";
import { DeviceData } from "./DeviceData";
import { getByClass } from "./DeviceSummaryManagedFactory";

const DeviceSummaryManagedWrapper: React.FC<{
  deviceId: DeviceId;
  deviceData: DeviceData;
}> = (props) => {
  const { deviceId, deviceData } = props;

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
        <Component deviceId={deviceId} />
      </ComponentWrapper>
    </Wrapper>
  );
};
export default DeviceSummaryManagedWrapper;

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

  overflow: hidden;
  text-overflow: ellipsis;

  word-break: keep-all;
  white-space: nowrap;
`;

const ComponentWrapper = styled.div``;
