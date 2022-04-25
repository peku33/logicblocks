import Colors from "components/common/Colors";
import styled from "styled-components";
import SummaryManagedWrapper from "./SummaryManagedWrapper";

const SummaryManagedWrapperList: React.VFC<{
  deviceIds: number[];
}> = (props) => {
  const { deviceIds } = props;

  return (
    <Grid>
      {deviceIds.map((deviceId) => (
        <GridItem key={deviceId}>
          <SummaryManagedWrapper deviceId={deviceId} />
        </GridItem>
      ))}
    </Grid>
  );
};
export default SummaryManagedWrapperList;

const Grid = styled.div`
  margin: 0.25rem;

  display: grid;
  grid-gap: 0.25rem;

  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  grid-auto-rows: auto;

  align-items: start;
  justify-content: center;
`;
const GridItem = styled.div`
  padding: 0.25rem;

  border: solid 1px ${Colors.GREY};
`;
