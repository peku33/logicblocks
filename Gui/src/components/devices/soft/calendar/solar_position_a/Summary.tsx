import Colors from "components/common/Colors";
import { formatDegreesOrUndefined } from "datatypes/Angle";
import { formatRealOrUndefined } from "datatypes/Real";
import styled from "styled-components";

export interface DataInner {
  julian_day: number;
  elevation: number;
  asimuth: number;
}
export type Data = DataInner | null;

const Component: React.VFC<{
  data: Data | undefined;
}> = (props) => {
  const { data } = props;

  return (
    <Wrapper>
      <Table>
        <TableHeader>
          <TableRow>
            <TableCell>Julian Day</TableCell>
            <TableCell>Elevation</TableCell>
            <TableCell>Asimuth</TableCell>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow>
            <TableCell>{formatRealOrUndefined(data?.julian_day, 2)}</TableCell>
            <TableCell>{formatDegreesOrUndefined(data?.elevation, 2)}</TableCell>
            <TableCell>{formatDegreesOrUndefined(data?.asimuth, 2)}</TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;

const Table = styled.table`
  width: 100%;
  border-collapse: collapse;
  text-align: center;
`;

const TableHeader = styled.thead`
  font-size: small;
  font-weight: bold;
`;
const TableBody = styled.thead``;

const TableRow = styled.tr``;
const TableCell = styled.td`
  padding: 0.25rem 0.5rem;
  border: solid 1px ${Colors.GREY_LIGHTEST};
`;
