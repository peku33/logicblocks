import styled from "styled-components";
import Colors from "./Colors";

export const Line: React.VFC<{}> = (props) => {
  return <LineInner />;
};
const LineInner = styled.div`
  margin-bottom: 0.125rem;
  padding-bottom: 0.125rem;

  border-bottom: solid 1px ${Colors.GREY_LIGHT};

  margin-bottom: 0.25rem;
  padding-bottom: 0.25rem;
`;
