import { ButtonActionAsync, ButtonGroup } from "@/components/common/Button";
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
      <ButtonGroup>
        <ButtonActionAsync active={data !== undefined ? !data : false} onClick={onR}>
          Off
        </ButtonActionAsync>
        <ButtonActionAsync active={false} onClick={onT}>
          Toggle
        </ButtonActionAsync>
        <ButtonActionAsync active={data !== undefined ? data : false} onClick={onS}>
          On
        </ButtonActionAsync>
      </ButtonGroup>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;
