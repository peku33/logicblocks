import { Property } from "csstype";
import ColorRgbBoolean from "datatypes/ColorRgbBoolean";
import Ds18x20State from "datatypes/Ds18x20";
import { kelvinToCelsius, kelvinToFahrenheit } from "datatypes/Temperature";
import Voltage from "datatypes/Voltage";
import styled from "styled-components";

export interface Data {
  status_led: ColorRgbBoolean;
  block_1_values: DataBlock1Value[]; // Block1Size
  block_2_values: DataBlock2Value[]; // Block2Size
  block_3_values: DataBlock3Value[]; // Block3Size
  block_4_values: DataBlock4Value[]; // Block4Size
}

export const Block1Size = 4;
export type DataBlock1Value =
  | DataBlock1ValueUnused
  | DataBlock1ValueAnalogIn
  | DataBlock1ValueDigitalIn
  | DataBlock1ValueDigitalOut;

export interface DataBlock1ValueUnused {
  function: "Unused";
}
export function dataBlock1ValueIsUnused(dataBlock1Value: DataBlock1Value): dataBlock1Value is DataBlock1ValueUnused {
  return dataBlock1Value.function === "Unused";
}
export interface DataBlock1ValueAnalogIn {
  function: "AnalogIn";
  value: Voltage | null;
}
export function dataBlock1ValueIsAnalogIn(
  dataBlock1Value: DataBlock1Value,
): dataBlock1Value is DataBlock1ValueAnalogIn {
  return dataBlock1Value.function === "AnalogIn";
}
export interface DataBlock1ValueDigitalIn {
  function: "DigitalIn";
  value: boolean | null;
}
export function dataBlock1ValueIsDigitalIn(
  dataBlock1Value: DataBlock1Value,
): dataBlock1Value is DataBlock1ValueDigitalIn {
  return dataBlock1Value.function === "DigitalIn";
}
export interface DataBlock1ValueDigitalOut {
  function: "DigitalOut";
  value: boolean;
}
export function dataBlock1ValueIsDigitalOut(
  dataBlock1Value: DataBlock1Value,
): dataBlock1Value is DataBlock1ValueDigitalOut {
  return dataBlock1Value.function === "DigitalOut";
}

export const Block2Size = 4;
export type DataBlock2Value =
  | DataBlock2ValueUnused
  | DataBlock2ValueDigitalIn
  | DataBlock2ValueDigitalOut
  | DataBlock2ValueDs18x20;
export interface DataBlock2ValueUnused {
  function: "Unused";
}
export function dataBlock2ValueIsUnused(dataBlock2Value: DataBlock2Value): dataBlock2Value is DataBlock2ValueUnused {
  return dataBlock2Value.function === "Unused";
}
export interface DataBlock2ValueDigitalIn {
  function: "DigitalIn";
  value: boolean | null;
}
export function dataBlock2ValueIsDigitalIn(
  dataBlock2Value: DataBlock2Value,
): dataBlock2Value is DataBlock2ValueDigitalIn {
  return dataBlock2Value.function === "DigitalIn";
}
export interface DataBlock2ValueDigitalOut {
  function: "DigitalOut";
  value: boolean;
}
export function dataBlock2ValueIsDigitalOut(
  dataBlock2Value: DataBlock2Value,
): dataBlock2Value is DataBlock2ValueDigitalOut {
  return dataBlock2Value.function === "DigitalOut";
}
export interface DataBlock2ValueDs18x20 {
  function: "Ds18x20";
  value: Ds18x20State | null;
}
export function dataBlock2ValueIsDs18x20(dataBlock2Value: DataBlock2Value): dataBlock2Value is DataBlock2ValueDs18x20 {
  return dataBlock2Value.function === "Ds18x20";
}

export const Block3Size = 2;
export type DataBlock3Value = DataBlock3ValueUnused | DataBlock3ValueAnalogIn;
export interface DataBlock3ValueUnused {
  function: "Unused";
}
export function dataBlock3ValueIsUnused(dataBlock3Value: DataBlock3Value): dataBlock3Value is DataBlock3ValueUnused {
  return dataBlock3Value.function === "Unused";
}
export interface DataBlock3ValueAnalogIn {
  function: "AnalogIn";
  value: Voltage | null;
}
export function dataBlock3ValueIsAnalogIn(
  dataBlock3Value: DataBlock3Value,
): dataBlock3Value is DataBlock3ValueAnalogIn {
  return dataBlock3Value.function === "AnalogIn";
}

export const Block4Size = 3;
export type DataBlock4Value = DataBlock4ValueUnused | DataBlock4ValueDigitalOut;
export interface DataBlock4ValueUnused {
  function: "Unused";
}
export function dataBlock4ValueIsUnused(dataBlock4Value: DataBlock4Value): dataBlock4Value is DataBlock4ValueUnused {
  return dataBlock4Value.function === "Unused";
}
export interface DataBlock4ValueDigitalOut {
  function: "DigitalOut";
  value: boolean;
}
export function dataBlock4ValueIsDigitalOut(
  dataBlock4Value: DataBlock4Value,
): dataBlock4Value is DataBlock4ValueDigitalOut {
  return dataBlock4Value.function === "DigitalOut";
}

const Component: React.FC<{
  data: Data | undefined;
}> = (props) => {
  const { data } = props;

  return (
    <Layout>
      <LayoutItem column={6} row={1}>
        <LayoutItemStatusLed value={data?.status_led} />
      </LayoutItem>
      <LayoutItem column={1} row={1}>
        <LayoutItemBlock1 pin={1} value={data?.block_1_values[0]} />
      </LayoutItem>
      <LayoutItem column={2} row={1}>
        <LayoutItemBlock1 pin={2} value={data?.block_1_values[1]} />
      </LayoutItem>
      <LayoutItem column={3} row={1}>
        <LayoutItemBlock1 pin={3} value={data?.block_1_values[2]} />
      </LayoutItem>
      <LayoutItem column={4} row={1}>
        <LayoutItemBlock1 pin={4} value={data?.block_1_values[3]} />
      </LayoutItem>
      <LayoutItem column={8} row={1}>
        <LayoutItemBlock2 pin={1} value={data?.block_2_values[0]} />
      </LayoutItem>
      <LayoutItem column={9} row={1}>
        <LayoutItemBlock2 pin={2} value={data?.block_2_values[1]} />
      </LayoutItem>
      <LayoutItem column={10} row={1}>
        <LayoutItemBlock2 pin={3} value={data?.block_2_values[2]} />
      </LayoutItem>
      <LayoutItem column={11} row={1}>
        <LayoutItemBlock2 pin={4} value={data?.block_2_values[3]} />
      </LayoutItem>
      <LayoutItem column={1} row={3}>
        <LayoutItemBlock3 pin={1} value={data?.block_3_values[0]} />
      </LayoutItem>
      <LayoutItem column={2} row={3}>
        <LayoutItemBlock3 pin={2} value={data?.block_3_values[1]} />
      </LayoutItem>
      <LayoutItem column={9} row={3}>
        <LayoutItemBlock4 pin={1} value={data?.block_4_values[0]} />
      </LayoutItem>
      <LayoutItem column={10} row={3}>
        <LayoutItemBlock4 pin={2} value={data?.block_4_values[1]} />
      </LayoutItem>
      <LayoutItem column={11} row={3}>
        <LayoutItemBlock4 pin={3} value={data?.block_4_values[2]} />
      </LayoutItem>
    </Layout>
  );
};
export default Component;

const Layout = styled.div`
  display: grid;

  grid-template-columns: repeat(auto-fit, minmax(4rem, 1fr));
  grid-auto-rows: 1fr;
  grid-gap: 0.125rem;
`;
const LayoutItem = styled.div<{
  column: number;
  row: number;
}>`
  display: flex;
  align-items: center;
  justify-content: center;
  aspect-ratio: 1;
`;

const LayoutItemStatusLed: React.FC<{
  value: ColorRgbBoolean | undefined;
}> = (props) => {
  const { value } = props;

  return <LayoutItemStatusLedInner r={value?.r || false} g={value?.g || false} b={value?.b || false} />;
};
const LayoutItemStatusLedInner = styled.div<{
  r: boolean;
  g: boolean;
  b: boolean;
}>`
  width: 100%;
  height: 100%;

  background-color: ${({ r, g, b }) => `rgb(${r ? 255 : 0}, ${g ? 255 : 0}, ${b ? 255 : 0})`};
`;

const LayoutItemBlock1: React.FC<{
  pin: number; // 1-base
  value: DataBlock1Value | undefined;
}> = (props) => {
  const { pin, value } = props;

  if (value === undefined || dataBlock1ValueIsUnused(value)) {
    return <LayoutItemUnused block={1} pin={pin} />;
  } else if (dataBlock1ValueIsAnalogIn(value)) {
    return <LayoutItemAnalogIn block={1} pin={pin} voltage={value.value} />;
  } else if (dataBlock1ValueIsDigitalIn(value)) {
    return <LayoutItemDigitalIn block={1} pin={pin} value={value.value} />;
  } else if (dataBlock1ValueIsDigitalOut(value)) {
    return <LayoutItemDigitalOut block={1} pin={pin} value={value.value} />;
  } else {
    throw new Error("unknown value type");
  }
};
const LayoutItemBlock2: React.FC<{
  pin: number; // 1-base
  value: DataBlock2Value | undefined;
}> = (props) => {
  const { pin, value } = props;

  if (value === undefined || dataBlock2ValueIsUnused(value)) {
    return <LayoutItemUnused block={2} pin={pin} />;
  } else if (dataBlock2ValueIsDigitalIn(value)) {
    return <LayoutItemDigitalIn block={2} pin={pin} value={value.value} />;
  } else if (dataBlock2ValueIsDigitalOut(value)) {
    return <LayoutItemDigitalOut block={2} pin={pin} value={value.value} />;
  } else if (dataBlock2ValueIsDs18x20(value)) {
    return <LayoutItemDs18x20 block={2} pin={pin} state={value.value} />;
  } else {
    throw new Error("unknown value type");
  }
};
const LayoutItemBlock3: React.FC<{
  pin: number; // 1-base
  value: DataBlock3Value | undefined;
}> = (props) => {
  const { pin, value } = props;

  if (value === undefined || dataBlock3ValueIsUnused(value)) {
    return <LayoutItemUnused block={3} pin={pin} />;
  } else if (dataBlock3ValueIsAnalogIn(value)) {
    return <LayoutItemAnalogIn block={3} pin={pin} voltage={value.value} />;
  } else {
    throw new Error("unknown value type");
  }
};
const LayoutItemBlock4: React.FC<{
  pin: number; // 1-base
  value: DataBlock4Value | undefined;
}> = (props) => {
  const { pin, value } = props;

  if (value === undefined || dataBlock4ValueIsUnused(value)) {
    return <LayoutItemUnused block={4} pin={pin} />;
  } else if (dataBlock4ValueIsDigitalOut(value)) {
    return <LayoutItemDigitalOut block={4} pin={pin} value={value.value} />;
  } else {
    throw new Error("unknown value type");
  }
};

const LayoutItemUnused: React.FC<{
  block: number; // 1-based
  pin: number; // 1-base
}> = (props) => {
  const { block, pin } = props;

  return (
    <LayoutItemInner backgroundColor="lightgrey">
      <LayoutItemInnerBlockPinLabel block={block} pin={pin} />
      <LayoutItemInnerLabel>Unused</LayoutItemInnerLabel>
    </LayoutItemInner>
  );
};
const LayoutItemAnalogIn: React.FC<{
  block: number; // 1-based
  pin: number; // 1-base
  voltage: Voltage | null;
}> = (props) => {
  const { block, pin, voltage } = props;

  return (
    <LayoutItemInner backgroundColor="#FDCEB9">
      <LayoutItemInnerBlockPinLabel block={block} pin={pin} />
      <LayoutItemInnerLabel>Analog Input</LayoutItemInnerLabel>
      <LayoutItemInnerValue>{voltage !== null ? `${voltage.toFixed(4)}V` : "Unknown"}</LayoutItemInnerValue>
    </LayoutItemInner>
  );
};
const LayoutItemDigitalIn: React.FC<{
  block: number; // 1-based
  pin: number; // 1-base
  value: boolean | null;
}> = (props) => {
  const { block, pin, value } = props;

  return (
    <LayoutItemInner backgroundColor="#655D8A">
      <LayoutItemInnerBlockPinLabel block={block} pin={pin} />
      <LayoutItemInnerLabel>Digital Input</LayoutItemInnerLabel>
      <LayoutItemInnerValue>{value !== null ? (value ? "On" : "Off") : "Unknown"}</LayoutItemInnerValue>
    </LayoutItemInner>
  );
};
const LayoutItemDigitalOut: React.FC<{
  block: number; // 1-based
  pin: number; // 1-base
  value: boolean;
}> = (props) => {
  const { block, pin, value } = props;

  return (
    <LayoutItemInner backgroundColor="#7897AB">
      <LayoutItemInnerBlockPinLabel block={block} pin={pin} />
      <LayoutItemInnerLabel>Digital Output</LayoutItemInnerLabel>
      <LayoutItemInnerValue>{value ? "On" : "Off"}</LayoutItemInnerValue>
    </LayoutItemInner>
  );
};
const LayoutItemDs18x20: React.FC<{
  block: number; // 1-based
  pin: number; // 1-base
  state: Ds18x20State | null;
}> = (props) => {
  const { block, pin, state } = props;

  return (
    <LayoutItemInner backgroundColor="#D885A3">
      <LayoutItemInnerBlockPinLabel block={block} pin={pin} />
      <LayoutItemInnerLabel>DS18x20</LayoutItemInnerLabel>
      {state !== null ? (
        <>
          {state.temperature != null ? (
            <>
              <LayoutItemInnerValue>{kelvinToCelsius(state.temperature).toFixed(2)}&deg;C</LayoutItemInnerValue>
              <LayoutItemInnerValue>{kelvinToFahrenheit(state.temperature).toFixed(2)}&deg;F</LayoutItemInnerValue>
            </>
          ) : null}
          <LayoutItemInnerValue>
            {state.sensor_type} ({state.reset_count})
          </LayoutItemInnerValue>
        </>
      ) : (
        <LayoutItemInnerValue>Unknown</LayoutItemInnerValue>
      )}
    </LayoutItemInner>
  );
};

const LayoutItemInner = styled.div<{
  backgroundColor: Property.Color;
}>`
  width: 100%;
  height: 100%;

  display: flex;
  flex-direction: column;

  align-items: center;
  justify-content: center;
  text-align: center;

  background-color: ${({ backgroundColor }) => backgroundColor};
`;
const LayoutItemInnerLabel = styled.div`
  font-size: x-small;
`;
const LayoutItemInnerValue = styled.div`
  font-size: small;
  font-weight: bold;
`;

const LayoutItemInnerBlockPinLabel: React.FC<{
  block: number; // 1-based
  pin: number; // 1-base
}> = (props) => {
  const { block, pin } = props;

  return (
    <LayoutItemInnerBlockPinLabelInner>
      B{block}.P{pin}
    </LayoutItemInnerBlockPinLabelInner>
  );
};
const LayoutItemInnerBlockPinLabelInner = styled.div`
  font-size: x-small;
`;
