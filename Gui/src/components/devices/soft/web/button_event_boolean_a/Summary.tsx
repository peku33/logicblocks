import { Button, ButtonGroup } from "components/common/Button";
import styled from "styled-components";

const Component: React.VFC<{
  onPush: (value: boolean) => void;
}> = (props) => {
  const { onPush } = props;

  return (
    <Wrapper>
      <ButtonGroup>
        <Button onMouseUp={() => onPush(false)}>Off</Button>
        <Button onMouseUp={() => onPush(true)}>On</Button>
      </ButtonGroup>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;
