import { ButtonGroup, ButtonPressRelease } from "@/components/common/Button";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { useCallback } from "react";
import styled from "styled-components";

export type Data = boolean | null;

const Component: React.FC<{
  data: Data | undefined;
  onValueChanged: (newValue: boolean | null) => void;
}> = (props) => {
  const { data, onValueChanged } = props;

  const onPressedChangedDown = useCallback(
    (value: boolean) => {
      onValueChanged(value ? false : null);
    },
    [onValueChanged],
  );
  const onPressedChangedUp = useCallback(
    (value: boolean) => {
      onValueChanged(value ? true : null);
    },
    [onValueChanged],
  );

  return (
    <Wrapper>
      <ButtonGroup>
        <ButtonPressRelease
          active={data !== undefined ? data === false : false}
          onPressedChanged={onPressedChangedDown}
        >
          <ButtonIconWrapper>
            <FontAwesomeIcon icon={{ prefix: "far", iconName: "square-caret-down" }} />
          </ButtonIconWrapper>
          <ButtonTextWrapper>Down</ButtonTextWrapper>
        </ButtonPressRelease>
        <ButtonPressRelease active={data !== undefined ? data === true : false} onPressedChanged={onPressedChangedUp}>
          <ButtonIconWrapper>
            <FontAwesomeIcon icon={{ prefix: "far", iconName: "square-caret-up" }} />
          </ButtonIconWrapper>
          <ButtonTextWrapper>Up</ButtonTextWrapper>
        </ButtonPressRelease>
      </ButtonGroup>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;

const ButtonIconWrapper = styled.span`
  margin-right: 0.5rem;
`;

const ButtonTextWrapper = styled.span``;
