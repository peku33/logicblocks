import MediaQueries from "components/common/MediaQueries";
import { Property } from "csstype";
import ColorRgbBoolean from "datatypes/ColorRgbBoolean";
import Ds18x20State from "datatypes/Ds18x20";
import Voltage from "datatypes/Voltage";
import { kelvinToCelsius, kelvinToFahrenheit } from "lib/Temperature";
import styled from "styled-components";

interface DeviceSummaryBlock1ValueUnused {
  function: "Unused";
}
interface DeviceSummaryBlock1ValueAnalogIn {
  function: "AnalogIn";
  value: Voltage | null;
}
interface DeviceSummaryBlock1ValueDigitalIn {
  function: "DigitalIn";
  value: boolean | null;
}
interface DeviceSummaryBlock1ValueDigitalOut {
  function: "DigitalOut";
  value: boolean;
}
type DeviceSummaryBlock1Value =
  | DeviceSummaryBlock1ValueUnused
  | DeviceSummaryBlock1ValueAnalogIn
  | DeviceSummaryBlock1ValueDigitalIn
  | DeviceSummaryBlock1ValueDigitalOut;
function deviceSummaryBlock1ValueIsUnused(
  deviceSummaryBlock1Value: DeviceSummaryBlock1Value,
): deviceSummaryBlock1Value is DeviceSummaryBlock1ValueUnused {
  return deviceSummaryBlock1Value.function === "Unused";
}
function deviceSummaryBlock1ValueIsAnalogIn(
  deviceSummaryBlock1Value: DeviceSummaryBlock1Value,
): deviceSummaryBlock1Value is DeviceSummaryBlock1ValueAnalogIn {
  return deviceSummaryBlock1Value.function === "AnalogIn";
}
function deviceSummaryBlock1ValueIsDigitalIn(
  deviceSummaryBlock1Value: DeviceSummaryBlock1Value,
): deviceSummaryBlock1Value is DeviceSummaryBlock1ValueDigitalIn {
  return deviceSummaryBlock1Value.function === "DigitalIn";
}
function deviceSummaryBlock1ValueIsDigitalOut(
  deviceSummaryBlock1Value: DeviceSummaryBlock1Value,
): deviceSummaryBlock1Value is DeviceSummaryBlock1ValueDigitalOut {
  return deviceSummaryBlock1Value.function === "DigitalOut";
}

interface DeviceSummaryBlock2ValueUnused {
  function: "Unused";
}
interface DeviceSummaryBlock2ValueDigitalIn {
  function: "DigitalIn";
  value: boolean | null;
}
interface DeviceSummaryBlock2ValueDigitalOut {
  function: "DigitalOut";
  value: boolean;
}
interface DeviceSummaryBlock2ValueDs18x20 {
  function: "Ds18x20";
  value: Ds18x20State | null;
}
type DeviceSummaryBlock2Value =
  | DeviceSummaryBlock2ValueUnused
  | DeviceSummaryBlock2ValueDigitalIn
  | DeviceSummaryBlock2ValueDigitalOut
  | DeviceSummaryBlock2ValueDs18x20;
function deviceSummaryBlock2ValueIsUnused(
  deviceSummaryBlock2Value: DeviceSummaryBlock2Value,
): deviceSummaryBlock2Value is DeviceSummaryBlock2ValueUnused {
  return deviceSummaryBlock2Value.function === "Unused";
}
function deviceSummaryBlock2ValueIsDigitalIn(
  deviceSummaryBlock2Value: DeviceSummaryBlock2Value,
): deviceSummaryBlock2Value is DeviceSummaryBlock2ValueDigitalIn {
  return deviceSummaryBlock2Value.function === "DigitalIn";
}
function deviceSummaryBlock2ValueIsDigitalOut(
  deviceSummaryBlock2Value: DeviceSummaryBlock2Value,
): deviceSummaryBlock2Value is DeviceSummaryBlock2ValueDigitalOut {
  return deviceSummaryBlock2Value.function === "DigitalOut";
}
function deviceSummaryBlock2ValueIsDs18x20(
  deviceSummaryBlock2Value: DeviceSummaryBlock2Value,
): deviceSummaryBlock2Value is DeviceSummaryBlock2ValueDs18x20 {
  return deviceSummaryBlock2Value.function === "Ds18x20";
}

interface DeviceSummaryBlock3ValueUnused {
  function: "Unused";
}
interface DeviceSummaryBlock3ValueAnalogIn {
  function: "AnalogIn";
  value: Voltage | null;
}
type DeviceSummaryBlock3Value = DeviceSummaryBlock3ValueUnused | DeviceSummaryBlock3ValueAnalogIn;
function deviceSummaryBlock3ValueIsUnused(
  deviceSummaryBlock3Value: DeviceSummaryBlock3Value,
): deviceSummaryBlock3Value is DeviceSummaryBlock3ValueUnused {
  return deviceSummaryBlock3Value.function === "Unused";
}
function deviceSummaryBlock3ValueIsAnalogIn(
  deviceSummaryBlock3Value: DeviceSummaryBlock3Value,
): deviceSummaryBlock3Value is DeviceSummaryBlock3ValueAnalogIn {
  return deviceSummaryBlock3Value.function === "AnalogIn";
}

interface DeviceSummaryBlock4ValueUnused {
  function: "Unused";
}
interface DeviceSummaryBlock4ValueDigitalOut {
  function: "DigitalOut";
  value: boolean;
}
type DeviceSummaryBlock4Value = DeviceSummaryBlock4ValueUnused | DeviceSummaryBlock4ValueDigitalOut;
function deviceSummaryBlock4ValueIsUnused(
  deviceSummaryBlock4Value: DeviceSummaryBlock4Value,
): deviceSummaryBlock4Value is DeviceSummaryBlock4ValueUnused {
  return deviceSummaryBlock4Value.function === "Unused";
}
function deviceSummaryBlock4ValueIsDigitalOut(
  deviceSummaryBlock4Value: DeviceSummaryBlock4Value,
): deviceSummaryBlock4Value is DeviceSummaryBlock4ValueDigitalOut {
  return deviceSummaryBlock4Value.function === "DigitalOut";
}

interface DeviceSummary {
  status_led: ColorRgbBoolean;
  block_1_values: DeviceSummaryBlock1Value[];
  block_2_values: DeviceSummaryBlock2Value[];
  block_3_values: DeviceSummaryBlock3Value[];
  block_4_values: DeviceSummaryBlock4Value[];
}

const LayoutItemInner = styled.div<{
  backgroundColor: Property.Color;
}>`
  width: 100%;
  height: 100%;

  display: flex;
  flex-direction: column;

  align-items: center;
  justify-content: center;

  background-color: ${({ backgroundColor }) => backgroundColor};
`;
const LayoutItemInnerLabel = styled.div`
  font-size: xx-small;
  text-align: center;

  @media ${MediaQueries.COMPUTER_AT_LEAST} {
    font-size: x-small;
  }
`;
const LayoutItemInnerValue = styled.div`
  font-size: x-small;
  text-align: center;
  font-weight: bold;

  @media ${MediaQueries.COMPUTER_AT_LEAST} {
    font-size: small;
  }
`;

const LayoutItemUnused: React.VFC<{}> = (props) => {
  return (
    <LayoutItemInner backgroundColor="lightgrey">
      <LayoutItemInnerLabel>Unused</LayoutItemInnerLabel>
    </LayoutItemInner>
  );
};
const LayoutItemAnalogIn: React.VFC<{
  voltage: Voltage | null;
}> = (props) => {
  const { voltage } = props;

  return (
    <LayoutItemInner backgroundColor="#FDCEB9">
      <LayoutItemInnerLabel>Analog Input</LayoutItemInnerLabel>
      <LayoutItemInnerValue>{voltage !== null ? `${voltage.toFixed(4)}V` : "Unknown"}</LayoutItemInnerValue>
    </LayoutItemInner>
  );
};
const LayoutItemDigitalIn: React.VFC<{
  value: boolean | null;
}> = (props) => {
  const { value } = props;

  return (
    <LayoutItemInner backgroundColor="#655D8A">
      <LayoutItemInnerLabel>Digital Input</LayoutItemInnerLabel>
      <LayoutItemInnerValue>{value !== null ? (value ? "On" : "Off") : "Unknown"}</LayoutItemInnerValue>
    </LayoutItemInner>
  );
};
const LayoutItemDigitalOut: React.VFC<{
  value: boolean;
}> = (props) => {
  const { value } = props;

  return (
    <LayoutItemInner backgroundColor="#7897AB">
      <LayoutItemInnerLabel>Digital Output</LayoutItemInnerLabel>
      <LayoutItemInnerValue>{value ? "On" : "Off"}</LayoutItemInnerValue>
    </LayoutItemInner>
  );
};
const LayoutItemDs18x20: React.VFC<{
  state: Ds18x20State | null;
}> = (props) => {
  const { state } = props;

  return (
    <LayoutItemInner backgroundColor="#D885A3">
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

const LayoutItemStatusLed: React.VFC<{
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

const LayoutItemBlock1: React.VFC<{
  value: DeviceSummaryBlock1Value | undefined;
}> = (props) => {
  const { value } = props;

  if (value === undefined || deviceSummaryBlock1ValueIsUnused(value)) {
    return <LayoutItemUnused />;
  } else if (deviceSummaryBlock1ValueIsAnalogIn(value)) {
    return <LayoutItemAnalogIn voltage={value.value} />;
  } else if (deviceSummaryBlock1ValueIsDigitalIn(value)) {
    return <LayoutItemDigitalIn value={value.value} />;
  } else if (deviceSummaryBlock1ValueIsDigitalOut(value)) {
    return <LayoutItemDigitalOut value={value.value} />;
  } else {
    throw new Error("unknown value type");
  }
};
const LayoutItemBlock2: React.VFC<{
  value: DeviceSummaryBlock2Value | undefined;
}> = (props) => {
  const { value } = props;

  if (value === undefined || deviceSummaryBlock2ValueIsUnused(value)) {
    return <LayoutItemUnused />;
  } else if (deviceSummaryBlock2ValueIsDigitalIn(value)) {
    return <LayoutItemDigitalIn value={value.value} />;
  } else if (deviceSummaryBlock2ValueIsDigitalOut(value)) {
    return <LayoutItemDigitalOut value={value.value} />;
  } else if (deviceSummaryBlock2ValueIsDs18x20(value)) {
    return <LayoutItemDs18x20 state={value.value} />;
  } else {
    throw new Error("unknown value type");
  }
};
const LayoutItemBlock3: React.VFC<{
  value: DeviceSummaryBlock3Value | undefined;
}> = (props) => {
  const { value } = props;

  if (value === undefined || deviceSummaryBlock3ValueIsUnused(value)) {
    return <LayoutItemUnused />;
  } else if (deviceSummaryBlock3ValueIsAnalogIn(value)) {
    return <LayoutItemAnalogIn voltage={value.value} />;
  } else {
    throw new Error("unknown value type");
  }
};
const LayoutItemBlock4: React.VFC<{
  value: DeviceSummaryBlock4Value | undefined;
}> = (props) => {
  const { value } = props;

  if (value === undefined || deviceSummaryBlock4ValueIsUnused(value)) {
    return <LayoutItemUnused />;
  } else if (deviceSummaryBlock4ValueIsDigitalOut(value)) {
    return <LayoutItemDigitalOut value={value.value} />;
  } else {
    throw new Error("unknown value type");
  }
};

const Summary: React.VFC<{
  summary: DeviceSummary | undefined;
}> = (props) => {
  const { summary } = props;
  return (
    <Layout>
      <LayoutItem column={6} row={1}>
        <LayoutItemStatusLed value={summary?.status_led} />
      </LayoutItem>
      <LayoutItem column={1} row={1}>
        <LayoutItemBlock1 value={summary?.block_1_values[0]} />
      </LayoutItem>
      <LayoutItem column={2} row={1}>
        <LayoutItemBlock1 value={summary?.block_1_values[1]} />
      </LayoutItem>
      <LayoutItem column={3} row={1}>
        <LayoutItemBlock1 value={summary?.block_1_values[2]} />
      </LayoutItem>
      <LayoutItem column={4} row={1}>
        <LayoutItemBlock1 value={summary?.block_1_values[3]} />
      </LayoutItem>
      <LayoutItem column={8} row={1}>
        <LayoutItemBlock2 value={summary?.block_2_values[0]} />
      </LayoutItem>
      <LayoutItem column={9} row={1}>
        <LayoutItemBlock2 value={summary?.block_2_values[1]} />
      </LayoutItem>
      <LayoutItem column={10} row={1}>
        <LayoutItemBlock2 value={summary?.block_2_values[2]} />
      </LayoutItem>
      <LayoutItem column={11} row={1}>
        <LayoutItemBlock2 value={summary?.block_2_values[3]} />
      </LayoutItem>
      <LayoutItem column={1} row={3}>
        <LayoutItemBlock3 value={summary?.block_3_values[0]} />
      </LayoutItem>
      <LayoutItem column={2} row={3}>
        <LayoutItemBlock3 value={summary?.block_3_values[1]} />
      </LayoutItem>
      <LayoutItem column={9} row={3}>
        <LayoutItemBlock4 value={summary?.block_4_values[0]} />
      </LayoutItem>
      <LayoutItem column={10} row={3}>
        <LayoutItemBlock4 value={summary?.block_4_values[1]} />
      </LayoutItem>
      <LayoutItem column={11} row={3}>
        <LayoutItemBlock4 value={summary?.block_4_values[2]} />
      </LayoutItem>
    </Layout>
  );
};
export default Summary;

const Layout = styled.div`
  display: grid;

  grid-template-columns: repeat(auto-fit, minmax(4rem, 1fr));
  grid-auto-rows: 1fr;
  grid-gap: 0.125rem;

  @media ${MediaQueries.COMPUTER_AT_LEAST} {
    grid-template-columns: repeat(11, 1fr);
    grid-template-rows: repeat(3, 1fr);
  }
`;
const LayoutItem = styled.div<{
  column: number;
  row: number;
}>`
  display: flex;
  align-items: center;
  justify-content: center;
  aspect-ratio: 1;

  @media ${MediaQueries.COMPUTER_AT_LEAST} {
    grid-area: ${({ column, row }) => `${row} / ${column} / ${row} / ${column}`};
  }
`;
