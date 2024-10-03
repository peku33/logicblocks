import { Chip, ChipsGroup, ChipType } from "@/components/common/Chips";
import GaugeLinear from "@/components/common/GaugeLinear";
import GaugeLinearRatio from "@/components/datatypes/ratio/GaugeLinear";
import { formatVoltage } from "@/datatypes/Voltage";
import { formatSI } from "@/util/Number";
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

  motor_voltage_max_v: number;
  motor_current_rated_a: number;
  motor_current_max_a: number;
  motor_frequency_min_hz: number;
  motor_frequency_max_hz: number;
  motor_frequency_rated_hz: number;
  motor_speed_rated_rpm: number;

  motor_voltage_v: number;
  motor_current_a: number;
  motor_frequency_hz: number;
  motor_speed_rpm: number;
  motor_torque: number;
  motor_power: number;

  dc_link_voltage_v: number;
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
                value={data.motor_voltage_v}
                valueMin={0.0}
                valueMax={data.motor_voltage_max_v}
                valueSerializer={voltageSerializer}
              >
                Voltage
              </GaugeLinear>
              <GaugeLinear
                value={data.motor_current_a}
                valueMin={0.0}
                valueMax={data.motor_current_rated_a}
                valueSerializer={currentSerializer}
              >
                Current
              </GaugeLinear>
              <GaugeLinear
                value={data.motor_frequency_hz}
                valueMin={data.motor_frequency_min_hz}
                valueMax={data.motor_frequency_max_hz}
                valueSerializer={frequencySerializer}
              >
                Frequency
              </GaugeLinear>
              <GaugeLinear
                value={data.motor_speed_rpm}
                valueMin={0}
                valueMax={data.motor_speed_rated_rpm}
                valueSerializer={rpmSerializer}
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
                value={data.dc_link_voltage_v}
                valueMin={0}
                valueMax={DC_LINK_VOLTAGE_MAX}
                valueSerializer={voltageSerializer}
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

function voltageSerializer(voltage: number): string {
  return formatVoltage(voltage, 2);
}
function currentSerializer(current: number): string {
  return formatSI(current, 2, "A");
}
function frequencySerializer(frequency: number): string {
  return formatSI(frequency, 1, "Hz");
}
function rpmSerializer(rpm: number): string {
  return formatSI(rpm, 0, "rpm");
}
