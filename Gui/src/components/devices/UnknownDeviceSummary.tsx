import styled from "styled-components";
import { useDeviceData } from "./DeviceDataService";
import { DeviceSummaryManaged } from "./DeviceSummaryManaged";

const Summary: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  const deviceData = useDeviceData(deviceId);

  return (
    <Wrapper>
      Unknown device #<DetailsSpan>{deviceId}</DetailsSpan> <DetailsSpan>({deviceData?.class})</DetailsSpan>
    </Wrapper>
  );
};
export default Summary;

const Wrapper = styled.div`
  font-size: x-small;
`;

const DetailsSpan = styled.span`
  word-break: break-all;
`;
