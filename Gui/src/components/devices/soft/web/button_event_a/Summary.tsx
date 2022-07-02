import { Button } from "components/common/Button";
import styled from "styled-components";

const Component: React.FC<{
  onSignal: () => void;
}> = (props) => {
  const { onSignal } = props;

  return (
    <Wrapper>
      <Button onClick={onSignal}>Signal</Button>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;
