import Colors from "components/common/Colors";
import styled from "styled-components";
import SummaryManagedWrapper from "./SummaryManagedWrapper";

const SummaryManagedWrapperList: React.VFC<{
  deviceIds: number[];
}> = (props) => {
  const { deviceIds } = props;

  return (
    <DevicesList>
      {deviceIds.map((deviceId) => (
        <DevicesListItem key={deviceId}>
          <SummaryManagedWrapper deviceId={deviceId} />
        </DevicesListItem>
      ))}
    </DevicesList>
  );
};
export default SummaryManagedWrapperList;

const DevicesList = styled.div``;
const DevicesListItem = styled.div`
  padding: 0.5rem;
  border-bottom: solid 1px ${Colors.GREY_LIGHTEST};
  &:last-child {
    border-bottom: none;
  }
`;
