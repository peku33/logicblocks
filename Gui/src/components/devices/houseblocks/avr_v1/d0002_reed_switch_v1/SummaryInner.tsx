import Colors from "components/common/Colors";
import Resistance, { formatResistanceOrUnknown } from "datatypes/Resistance";
import styled from "styled-components";

export const INPUT_COUNT = 40;

// infinite values are serialized to nulls in JSON
export type ResistanceInfinityAsNull = Resistance | null;

export interface Data {
  inputs: ResistanceInfinityAsNull[] | null;
}

const Component: React.FC<{
  data: Data | undefined;
}> = (props) => {
  const { data } = props;

  const inputsFixed = data?.inputs?.map((input) => fixResistanceInfinityAsNull(input));

  return (
    <Grid>
      {Array.from(Array(INPUT_COUNT).keys()).map((index) => (
        <GridItem key={index}>
          <GridItemLabel>#{index + 1}</GridItemLabel>
          <GridItemValue>{formatResistanceOrUnknown(inputsFixed?.[index], 2)}</GridItemValue>
        </GridItem>
      ))}
      <GridItemCenter />
    </Grid>
  );
};
export default Component;

function fixResistanceInfinityAsNull(input: ResistanceInfinityAsNull | undefined): Resistance | undefined {
  if (input === undefined) return undefined;
  if (input === null) return Infinity;
  return input;
}

const Grid = styled.div`
  display: grid;
  grid-auto-flow: column;

  grid-template-columns: repeat(3, minmax(4rem, 1fr));
  grid-template-rows: repeat(20, 1fr);

  grid-gap: 0.25rem;
`;
const GridItem = styled.div`
  display: flex;
  flex-direction: column;
  padding: 0.25rem 0.5rem;

  align-items: center;
  justify-content: center;

  border: solid 1px ${Colors.GREY_LIGHT};
`;

const GridItemLabel = styled.div`
  font-size: x-small;
`;
const GridItemValue = styled.div`
  font-size: small;
  font-weight: bold;
`;

const GridItemCenter = styled.div`
  grid-column: 2;
  grid-row: span 20;

  background-color: ${Colors.GREY_LIGHTEST};
`;
