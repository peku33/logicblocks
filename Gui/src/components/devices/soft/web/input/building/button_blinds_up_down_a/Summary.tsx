import { Button, ButtonGroup } from "@/components/common/Button";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import styled from "styled-components";

export type Data = boolean | null;

const Component: React.FC<{
  data: Data | undefined;
  onValueChanged: (newValue: boolean | null) => void;
}> = (props) => {
  const { data, onValueChanged } = props;

  return (
    <Wrapper>
      <ButtonGroup>
        <Button
          active={data !== undefined ? data === false : undefined}
          onMouseDown={() => {
            onValueChanged(false);
          }}
          onMouseUp={() => {
            onValueChanged(null);
          }}
        >
          <ButtonIconWrapper>
            <FontAwesomeIcon icon={{ prefix: "far", iconName: "square-caret-down" }} />
          </ButtonIconWrapper>
          <ButtonTextWrapper>Down</ButtonTextWrapper>
        </Button>
        <Button
          active={data !== undefined ? data === true : undefined}
          onMouseDown={() => {
            onValueChanged(true);
          }}
          onMouseUp={() => {
            onValueChanged(null);
          }}
        >
          <ButtonIconWrapper>
            <FontAwesomeIcon icon={{ prefix: "far", iconName: "square-caret-up" }} />
          </ButtonIconWrapper>
          <ButtonTextWrapper>Up</ButtonTextWrapper>
        </Button>
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
