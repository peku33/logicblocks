import { Button } from "components/common/Button";
import styled from "styled-components";

const Summary: React.VFC<{
  onSignal: (() => void) | undefined;
}> = (props) => {
  const { onSignal } = props;

  return (
    <Wrapper>
      <Button onClick={onSignal !== undefined ? onSignal : () => ({})}>Signal</Button>
    </Wrapper>
  );
};
export default Summary;

const Wrapper = styled.div``;
