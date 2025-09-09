import { Chip, ChipType } from "@/components/common/Chips";
import {
  type Temperature,
  formatTemperatureCelsiusOrUnknown,
  formatTemperatureFahrenheitOrUnknown,
} from "@/datatypes/Temperature";
import styled from "styled-components";

export const KEYS_COUNT = 6;
export const LEDS_COUNT = 6;

export interface Data {
  keys: boolean[] | null;
  leds: boolean[];
  temperature: Temperature | null;
}

const Component: React.FC<{
  data: Data | undefined;
}> = (props) => {
  const { data } = props;

  return (
    <Wrapper>
      <KeysLedsWrapper>
        <SectionLabel>Keys:</SectionLabel>
        <KeysLedsValues>
          {Array.from(Array(KEYS_COUNT).keys()).map((index) => (
            <Chip key={index} type={ChipType.INFO} enabled={data?.keys?.at(index) || false}>
              {(index + 1).toString()}
            </Chip>
          ))}
        </KeysLedsValues>
      </KeysLedsWrapper>
      <KeysLedsWrapper>
        <SectionLabel>Leds:</SectionLabel>
        <KeysLedsValues>
          {Array.from(Array(LEDS_COUNT).keys()).map((index) => (
            <Chip key={index} type={ChipType.WARNING} enabled={data?.leds.at(index) || false}>
              {(index + 1).toString()}
            </Chip>
          ))}
        </KeysLedsValues>
      </KeysLedsWrapper>
      <TemperatureWrapper>
        <SectionLabel>Temperature:</SectionLabel>
        <TemperatureValuePrimary>{formatTemperatureCelsiusOrUnknown(data?.temperature, 2)}</TemperatureValuePrimary>
        <TemperatureValueSecondary>
          {formatTemperatureFahrenheitOrUnknown(data?.temperature, 2)}
        </TemperatureValueSecondary>
      </TemperatureWrapper>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div`
  display: grid;
  grid-gap: 0.5rem;
`;

const SectionLabel = styled.div`
  font-size: 0.75rem;
  text-align: right;
`;

const KeysLedsWrapper = styled.div``;
const KeysLedsValues = styled.div`
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: 1fr;
  grid-gap: 0.25rem;
`;

const TemperatureWrapper = styled.div``;
const TemperatureValuePrimary = styled.div`
  font-size: 1.25rem;
  font-weight: bold;
  text-align: right;
`;
const TemperatureValueSecondary = styled.div`
  font-size: 0.75rem;
  font-weight: bold;
  text-align: right;
`;
