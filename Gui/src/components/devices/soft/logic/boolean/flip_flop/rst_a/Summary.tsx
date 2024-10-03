import { Button, ButtonActionAsync } from "@/components/common/Button";
import styled from "styled-components";

export type Data = boolean;

const Component: React.FC<{
  data: Data | undefined;
  onR: () => Promise<void>;
  onS: () => Promise<void>;
  onT: () => Promise<void>;
}> = (props) => {
  const { data, onR, onS, onT } = props;

  return (
    <Wrapper>
      <Button>
        <ButtonActionAsync active={data !== undefined ? !data : undefined} onClick={onR}>
          Off
        </ButtonActionAsync>
        <ButtonActionAsync onClick={onT}>Toggle</ButtonActionAsync>
        <ButtonActionAsync active={data !== undefined ? data : undefined} onClick={onS}>
          On
        </ButtonActionAsync>
      </Button>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;
