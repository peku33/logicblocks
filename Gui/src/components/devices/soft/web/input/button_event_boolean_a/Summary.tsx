import { ButtonActionAsync, ButtonGroup } from "@/components/common/Button";
import styled from "styled-components";

export type Data = boolean;

const Component: React.FC<{
  data: Data | undefined;
  onPush: (value: boolean) => Promise<void>;
}> = (props) => {
  const { data, onPush } = props;

  return (
    <Wrapper>
      <ButtonGroup>
        <ButtonActionAsync
          active={data !== undefined ? !data : undefined}
          onClick={async () => {
            await onPush(false);
          }}
        >
          Off
        </ButtonActionAsync>
        <ButtonActionAsync
          active={data !== undefined ? data : undefined}
          onClick={async () => {
            await onPush(true);
          }}
        >
          On
        </ButtonActionAsync>
      </ButtonGroup>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;
