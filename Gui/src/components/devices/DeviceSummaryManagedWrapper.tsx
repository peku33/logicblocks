import { Line } from "components/common/Line";
import styled from "styled-components";
import { useDeviceData } from "./Device";
import { DeviceSummaryContext } from "./DeviceSummary";
import { getByClass } from "./DeviceSummaryManagedFactory";

const DeviceSummaryManagedWrapper: React.FC<{
  deviceSummaryContext: DeviceSummaryContext;
}> = (props) => {
  const { deviceSummaryContext } = props;
  const { deviceId } = deviceSummaryContext;

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
        <Component deviceSummaryContext={deviceSummaryContext} />
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
  font-size: normal;
`;

const ComponentWrapper = styled.div``;
