import { Button, ButtonGroup } from "components/common/Button";
import styled from "styled-components";

const Summary: React.VFC<{
  onPush: ((value: boolean) => void) | undefined;
}> = (props) => {
  const { onPush } = props;

  return (
    <Wrapper>
      <ButtonGroup>
        <Button onMouseUp={onPush !== undefined ? () => onPush(true) : () => ({})}>On</Button>
        <Button onMouseUp={onPush !== undefined ? () => onPush(false) : () => ({})}>Off</Button>
      </ButtonGroup>
    </Wrapper>
  );
};
export default Summary;

const Wrapper = styled.div``;
