import { Button, ButtonGroup } from "components/common/Button";
import styled from "styled-components";

export type Data = boolean;

const Component: React.VFC<{
  data: Data | undefined;
  onPush: (value: boolean) => void;
}> = (props) => {
  const { data, onPush } = props;

  return (
    <Wrapper>
      <ButtonGroup>
        <Button active={data === false} onClick={() => onPush(false)}>
          Off
        </Button>
        <Button active={data === true} onClick={() => onPush(true)}>
          On
        </Button>
      </ButtonGroup>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;
