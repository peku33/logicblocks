import { Button } from "components/common/Button";
import styled from "styled-components";

export type Data = boolean;

const Component: React.VFC<{
  data: Data | undefined;
  onValueChanged: (newValue: boolean) => void;
}> = (props) => {
  const { data, onValueChanged } = props;

  return (
    <Wrapper>
      <Button active={data} onMouseDown={() => onValueChanged(true)} onMouseUp={() => onValueChanged(false)}>
        Signal
      </Button>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;
