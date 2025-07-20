import { ButtonActionAsync, ButtonGroup } from "@/components/common/Button";
import styled from "styled-components";

export interface Data {
  input_value: boolean | null;
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

const Component: React.FC<{
  data: Data | undefined;
  onModeSet: (mode: boolean | null) => Promise<void>; // true/false = Override, null = PassThrough
  onModeCyclePassThrough: () => Promise<void>;
  onModeCycleNoPassThrough: () => Promise<void>;
}> = (props) => {
  const { data, onModeSet, onModeCyclePassThrough, onModeCycleNoPassThrough } = props;

  return (
    <Wrapper>
      <ButtonGroup>
        <ButtonActionAsync
          active={data !== undefined ? (dataModeIsOverride(data.mode) ? !data.mode.value : false) : false}
          onClick={async () => {
            await onModeSet(false);
          }}
        >
          <ButtonContent>
            <ButtonContentPrimary>Off</ButtonContentPrimary>
          </ButtonContent>
        </ButtonActionAsync>
        <ButtonActionAsync
          active={data !== undefined ? dataModeIsPassThrough(data.mode) : false}
          onClick={async () => {
            await onModeSet(null);
          }}
        >
          <ButtonContent>
            <ButtonContentPrimary>Auto</ButtonContentPrimary>
            <ButtonContentSecondary>
              {data !== undefined ? (
                <>({data.input_value === null ? "Unknown" : data.input_value ? "On" : "Off"})</>
              ) : null}
            </ButtonContentSecondary>
          </ButtonContent>
        </ButtonActionAsync>
        <ButtonActionAsync
          active={data !== undefined ? (dataModeIsOverride(data.mode) ? data.mode.value : false) : false}
          onClick={async () => {
            await onModeSet(true);
          }}
        >
          <ButtonContent>
            <ButtonContentPrimary>On</ButtonContentPrimary>
          </ButtonContent>
        </ButtonActionAsync>
      </ButtonGroup>
      <ButtonGroup>
        <ButtonActionAsync
          active={false}
          onClick={async () => {
            await onModeCyclePassThrough();
          }}
        >
          <ButtonContent>
            <ButtonContentPrimary>Cycle</ButtonContentPrimary>
            <ButtonContentSecondary>(With Auto)</ButtonContentSecondary>
          </ButtonContent>
        </ButtonActionAsync>
        <ButtonActionAsync
          active={false}
          onClick={async () => {
            await onModeCycleNoPassThrough();
          }}
        >
          <ButtonContent>
            <ButtonContentPrimary>Cycle</ButtonContentPrimary>
            <ButtonContentSecondary>(Skip Auto)</ButtonContentSecondary>
          </ButtonContent>
        </ButtonActionAsync>
      </ButtonGroup>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div`
  display: grid;

  grid-gap: 0.25rem;

  /* justify-items: center; */
  align-items: center;
`;

const ButtonContent = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
`;
const ButtonContentPrimary = styled.div``;
const ButtonContentSecondary = styled.div`
  font-size: small;
`;
