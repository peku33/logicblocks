import { ButtonPressRelease } from "@/components/common/Button";
import styled from "styled-components";

export type Data = boolean;

const Component: React.FC<{
  data: Data | undefined;
  onValueChanged: (newValue: boolean) => void;
}> = (props) => {
  const { data, onValueChanged } = props;

  return (
    <Wrapper>
      <ButtonPressRelease active={data !== undefined ? data : false} onPressedChanged={onValueChanged}>
        Signal
      </ButtonPressRelease>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;
