import { Button, ButtonGroup } from "components/common/Button";
import MediaQueries from "components/common/MediaQueries";
import React from "react";
import styled from "styled-components";

export interface Data {
  input_value: boolean;
  mode: DataMode;
}

export type DataMode = DataModePassThrough | DataModeOverride;
export interface DataModePassThrough {
  mode: "PassThrough";
}
export function dataModeIsPassThrough(dataMode: DataMode): dataMode is DataModePassThrough {
  return dataMode.mode === "PassThrough";
}
export interface DataModeOverride {
  mode: "Override";
  value: boolean;
}
export function dataModeIsOverride(dataMode: DataMode): dataMode is DataModeOverride {
  return dataMode.mode === "Override";
}

const Component: React.VFC<{
  data: Data | undefined;
  onModeSet: (mode: boolean | null) => void; // true/false = Override, null = PassThrough
  onModeCycle: () => void;
}> = (props) => {
  const { data, onModeSet, onModeCycle } = props;

  return (
    <Wrapper>
      <ButtonGroup>
        <Button
          active={data !== undefined ? (dataModeIsOverride(data.mode) ? !data.mode.value : false) : undefined}
          onClick={() => onModeSet(false)}
        >
          <ButtonContent>
            <ButtonContentPrimary>Off</ButtonContentPrimary>
          </ButtonContent>
        </Button>
        <Button
          active={data !== undefined ? dataModeIsPassThrough(data.mode) : undefined}
          onClick={() => onModeSet(null)}
        >
          <ButtonContent>
            <ButtonContentPrimary>Auto</ButtonContentPrimary>
            <ButtonContentSeconday>
              {data !== undefined ? <>({data.input_value ? "On" : "Off"})</> : null}
            </ButtonContentSeconday>
          </ButtonContent>
        </Button>
        <Button
          active={data !== undefined ? (dataModeIsOverride(data.mode) ? data.mode.value : false) : undefined}
          onClick={() => onModeSet(true)}
        >
          <ButtonContent>
            <ButtonContentPrimary>On</ButtonContentPrimary>
          </ButtonContent>
        </Button>
      </ButtonGroup>
      <Button onClick={() => onModeCycle()}>Cycle</Button>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div`
  display: grid;

  grid-gap: 0.25rem;

  /* justify-items: center; */
  align-items: center;

  @media ${MediaQueries.COMPUTER_AT_LEAST} {
    grid-gap: 0.5rem;
  }
`;

const ButtonContent = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
`;
const ButtonContentPrimary = styled.div``;
const ButtonContentSeconday = styled.div`
  font-size: small;
`;
