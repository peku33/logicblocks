import { Button } from "components/common/Button";
import styled from "styled-components";

const Summary: React.VFC<{
  value: boolean | undefined;
  onValueChanged: ((newValue: boolean) => void) | undefined;
}> = (props) => {
  const { value, onValueChanged } = props;

  return (
    <Wrapper>
      <Button
        active={value}
        onMouseDown={onValueChanged !== undefined ? () => onValueChanged(true) : () => ({})}
        onMouseUp={onValueChanged !== undefined ? () => onValueChanged(false) : () => ({})}
      >
        Signal
      </Button>
    </Wrapper>
  );
};
export default Summary;

const Wrapper = styled.div``;
