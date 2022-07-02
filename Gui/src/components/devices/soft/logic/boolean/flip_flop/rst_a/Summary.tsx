import { Button, ButtonGroup } from "components/common/Button";
import styled from "styled-components";

export type Data = boolean;

const Component: React.FC<{
  data: Data | undefined;
  onR: () => void;
  onS: () => void;
  onT: () => void;
}> = (props) => {
  const { data, onR, onS, onT } = props;

  return (
    <Wrapper>
      <ButtonGroup>
        <Button active={data !== undefined ? !data : undefined} onClick={onR}>
          Off
        </Button>
        <Button onClick={onT}>Toggle</Button>
        <Button active={data !== undefined ? data : undefined} onClick={onS}>
          On
        </Button>
      </ButtonGroup>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;
