import { Chip, ChipsGroup, ChipType } from "@/components/common/Chips";
import GaugeLinear from "@/components/common/GaugeLinear";
import GaugeLinearRatio from "@/components/datatypes/ratio/GaugeLinear";
import { formatCurrent, type Current } from "@/datatypes/Current";
import { formatFrequencyHertz, formatFrequencyRpm, type Frequency } from "@/datatypes/Frequency";
import { formatVoltage, type Voltage } from "@/datatypes/Voltage";
import styled from "styled-components";

const DC_LINK_VOLTAGE_MAX = 400;

export type Data = DataInitializing | DataRunning | DataError;
export interface DataInitializing {
  state: "Initializing";
}
export function dataIsInitializing(data: Data): data is DataInitializing {
  return data.state === "Initializing";
}
export interface DataRunning {
  state: "Running";

  warning: number | null;

  speed_control_active: boolean;

  ready: boolean;
  running: boolean;
  speed_setpoint: number;
  speed_actual: number;
  reverse: boolean;

  motor_voltage_max: Voltage;
  motor_current_rated: Current;
  motor_current_max: Current;
  motor_frequency_min: Frequency;
  motor_frequency_max: Frequency;
  motor_frequency_rated: Frequency;
  motor_speed_rated: Frequency;

  motor_voltage: Voltage;
  motor_current: Current;
  motor_frequency: Frequency;
  motor_speed: Frequency;
  motor_torque: number;
  motor_power: number;

  dc_link_voltage: Voltage;
  remote_input: boolean;
}
export function dataIsRunning(data: Data): data is DataRunning {
  return data.state === "Running";
}
export interface DataError {
  state: "Error";
}
export function dataIsError(data: Data): data is DataError {
  return data.state === "Error";
}

const Component: React.FC<{ data: Data | undefined }> = (props) => {
  const { data } = props;

  return (
    <Wrapper>
      <Section>
        <SectionTitle>Device status</SectionTitle>
        <SectionContent>
          <ChipsGroup>
            <Chip type={ChipType.INFO} enabled={data !== undefined && dataIsInitializing(data)}>
              Initializing
            </Chip>
            <Chip type={ChipType.OK} enabled={data !== undefined && dataIsRunning(data)}>
              Running
            </Chip>
            <Chip type={ChipType.ERROR} enabled={data !== undefined && dataIsError(data)}>
              Error
            </Chip>
          </ChipsGroup>
        </SectionContent>
      </Section>
      {data && dataIsRunning(data) ? (
        <>
          <Section>
            <SectionTitle>Drive status</SectionTitle>
            <SectionContent>
              <ChipsGroup>
                <Chip type={ChipType.WARNING} enabled={data.warning !== null}>
                  Warning: {data.warning !== null ? data.warning : "None"}
                </Chip>
                <Chip type={ChipType.OK} enabled={data.ready}>
                  Ready
                </Chip>
                <Chip type={ChipType.INFO} enabled={data.running}>
                  Running
                </Chip>
              </ChipsGroup>
            </SectionContent>
            <SectionContent>
              <GaugeLinearRatio value={data.speed_setpoint}>Speed Setpoint</GaugeLinearRatio>
              <GaugeLinearRatio value={data.speed_actual}>Speed Actual</GaugeLinearRatio>
            </SectionContent>
            <SectionContent>
              <ChipsGroup>
                <Chip type={ChipType.INFO} enabled={data.reverse}>
                  Reverse
                </Chip>
                <Chip type={ChipType.INFO} enabled={data.speed_control_active}>
                  Speed Control Mode Active
                </Chip>
              </ChipsGroup>
            </SectionContent>
          </Section>
          <Section>
            <SectionTitle>Motor status</SectionTitle>
            <SectionContent>
              <GaugeLinear
                value={data.motor_voltage}
                valueMin={0.0}
                valueMax={data.motor_voltage_max}
                valueSerializer={(voltage) => formatVoltage(voltage, 2)}
              >
                Voltage
              </GaugeLinear>
              <GaugeLinear
                value={data.motor_current}
                valueMin={0.0}
                valueMax={data.motor_current_rated}
                valueSerializer={(current) => formatCurrent(current, 2)}
              >
                Current
              </GaugeLinear>
              <GaugeLinear
                value={data.motor_frequency}
                valueMin={data.motor_frequency_min}
                valueMax={data.motor_frequency_max}
                valueSerializer={(frequency) => formatFrequencyHertz(frequency, 2)}
              >
                Frequency
              </GaugeLinear>
              <GaugeLinear
                value={data.motor_speed}
                valueMin={0}
                valueMax={data.motor_speed_rated}
                valueSerializer={(frequency) => formatFrequencyRpm(frequency, 0)}
              >
                RPM
              </GaugeLinear>
              <GaugeLinearRatio value={data.motor_torque}>Torque</GaugeLinearRatio>
              <GaugeLinearRatio value={data.motor_power}>Power</GaugeLinearRatio>
            </SectionContent>
          </Section>
          <Section>
            <SectionTitle>Other</SectionTitle>
            <SectionContent>
              <GaugeLinear
                value={data.dc_link_voltage}
                valueMin={0}
                valueMax={DC_LINK_VOLTAGE_MAX}
                valueSerializer={(voltage) => formatVoltage(voltage, 2)}
              >
                DC Link Voltage
              </GaugeLinear>
            </SectionContent>
          </Section>
        </>
      ) : null}
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;

const Section = styled.div`
  margin-bottom: 0.5rem;
`;
const SectionTitle = styled.div`
  font-weight: bold;
`;
const SectionContent = styled.div`
  padding-left: 1rem;
  & > * {
    margin-bottom: 0.25rem;
  }
`;
