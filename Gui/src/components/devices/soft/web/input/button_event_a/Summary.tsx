import { ButtonActionAsync } from "@/components/common/Button";
import styled from "styled-components";

const Component: React.FC<{
  onSignal: () => Promise<void>;
}> = (props) => {
  const { onSignal } = props;

  return (
    <Wrapper>
      <ButtonActionAsync active={false} onClick={onSignal}>
        Signal
      </ButtonActionAsync>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;
