import { Button, ButtonGroup } from "components/common/Button";
import React from "react";
import styled from "styled-components";

const Summary: React.VFC<{
  value: boolean | undefined;
  onR: (() => void) | undefined;
  onS: (() => void) | undefined;
  onT: (() => void) | undefined;
}> = (props) => {
  const { value, onR, onS, onT } = props;

  return (
    <Wrapper>
      <ButtonGroup>
        <Button active={value !== undefined ? value : undefined} onClick={onS !== undefined ? onS : () => ({})}>
          SET (On)
        </Button>
        <Button onClick={onT !== undefined ? onT : () => ({})}>TOGGLE (Flip)</Button>
        <Button active={value !== undefined ? !value : undefined} onClick={onR !== undefined ? onR : () => ({})}>
          RESET (Off)
        </Button>
      </ButtonGroup>
    </Wrapper>
  );
};
export default Summary;

const Wrapper = styled.div``;
