import { Button, ButtonGroup } from "components/common/Button";
import React from "react";
import styled from "styled-components";

export interface DeviceSummary {
  value: boolean;
}

const Summary: React.VFC<{
  deviceSummary: DeviceSummary | undefined;
  onR: (() => void) | undefined;
  onS: (() => void) | undefined;
  onT: (() => void) | undefined;
}> = (props) => {
  const { deviceSummary, onR, onS, onT } = props;

  return (
    <Wrapper>
      <ButtonGroup>
        <Button
          active={deviceSummary !== undefined ? deviceSummary.value : undefined}
          onClick={onS !== undefined ? onS : () => ({})}
        >
          SET (On)
        </Button>
        <Button onClick={onT !== undefined ? onT : () => ({})}>TOGGLE (Flip)</Button>
        <Button
          active={deviceSummary !== undefined ? !deviceSummary.value : undefined}
          onClick={onR !== undefined ? onR : () => ({})}
        >
          RESET (Off)
        </Button>
      </ButtonGroup>
    </Wrapper>
  );
};
export default Summary;

const Wrapper = styled.div``;
