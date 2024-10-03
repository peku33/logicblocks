import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import styled from "styled-components";

export const Loader: React.FC<{
  sizeRem: number;
}> = (props) => {
  const { sizeRem } = props;
  return (
    <LoaderIconWrapper $sizeRem={sizeRem}>
      <FontAwesomeIcon icon={{ prefix: "fas", iconName: "spinner" }} spin />
    </LoaderIconWrapper>
  );
};
export default Loader;
const LoaderIconWrapper = styled.div<{
  $sizeRem: number;
}>`
  text-align: center;
  font-size: ${(props) => props.$sizeRem}rem;
`;
