import { Chip, ChipsGroup, ChipType } from "components/common/Chips";
import GaugeLinear from "components/common/GaugeLinear";
import GaugeLinearRatio from "components/datatypes/ratio/GaugeLinear";
import styled from "styled-components";

const DC_LINK_VOLTAGE_MAX = 400;

export interface DeviceSummaryInitializing {
  state: "Initializing";
}
export interface DeviceSummaryRunning {
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
export interface DeviceSummaryError {
  state: "Error";
}
export type DeviceSummary = DeviceSummaryInitializing | DeviceSummaryRunning | DeviceSummaryError;
export function deviceSummaryIsInitializing(deviceSummary: DeviceSummary): deviceSummary is DeviceSummaryInitializing {
  return deviceSummary.state === "Initializing";
}
export function deviceSummaryIsRunning(deviceSummary: DeviceSummary): deviceSummary is DeviceSummaryRunning {
  return deviceSummary.state === "Running";
}
export function deviceSummaryIsError(deviceSummary: DeviceSummary): deviceSummary is DeviceSummaryError {
  return deviceSummary.state === "Error";
}

const Summary: React.VFC<{ deviceSummary: DeviceSummary | undefined }> = (props) => {
  const { deviceSummary } = props;

  return (
    <Wrapper>
      <Section>
        <SectionTitle>Device status</SectionTitle>
        <SectionContent>
          <ChipsGroup>
            <Chip
              type={ChipType.INFO}
              enabled={deviceSummary !== undefined && deviceSummaryIsInitializing(deviceSummary)}
            >
              Initializing
            </Chip>
            <Chip type={ChipType.OK} enabled={deviceSummary !== undefined && deviceSummaryIsRunning(deviceSummary)}>
              Running
            </Chip>
            <Chip type={ChipType.ERROR} enabled={deviceSummary !== undefined && deviceSummaryIsError(deviceSummary)}>
              Error
            </Chip>
          </ChipsGroup>
        </SectionContent>
      </Section>
      {deviceSummary && deviceSummaryIsRunning(deviceSummary) ? (
        <>
          <Section>
            <SectionTitle>Drive status</SectionTitle>
            <SectionContent>
              <ChipsGroup>
                <Chip type={ChipType.WARNING} enabled={deviceSummary.warning !== null}>
                  Warning: {deviceSummary.warning !== null ? deviceSummary.warning : "None"}
                </Chip>
                <Chip type={ChipType.OK} enabled={deviceSummary.ready}>
                  Ready
                </Chip>
                <Chip type={ChipType.INFO} enabled={deviceSummary.running}>
                  Running
                </Chip>
              </ChipsGroup>
            </SectionContent>
            <SectionContent>
              <GaugeLinearRatio value={deviceSummary.speed_setpoint}>Speed Setpoint</GaugeLinearRatio>
              <GaugeLinearRatio value={deviceSummary.speed_actual}>Speed Actual</GaugeLinearRatio>
            </SectionContent>
            <SectionContent>
              <ChipsGroup>
                <Chip type={ChipType.INFO} enabled={deviceSummary.reverse}>
                  Reverse
                </Chip>
                <Chip type={ChipType.INFO} enabled={deviceSummary.speed_control_active}>
                  Speed Control Mode Active
                </Chip>
              </ChipsGroup>
            </SectionContent>
          </Section>
          <Section>
            <SectionTitle>Motor status</SectionTitle>
            <SectionContent>
              <GaugeLinear
                value={deviceSummary.motor_voltage_v}
                valueMin={0.0}
                valueMax={deviceSummary.motor_voltage_max_v}
                valueSerializer={voltageSerializer}
              >
                Voltage
              </GaugeLinear>
              <GaugeLinear
                value={deviceSummary.motor_current_a}
                valueMin={0.0}
                valueMax={deviceSummary.motor_current_rated_a}
                valueSerializer={currentSerializer}
              >
                Current
              </GaugeLinear>
              <GaugeLinear
                value={deviceSummary.motor_frequency_hz}
                valueMin={deviceSummary.motor_frequency_min_hz}
                valueMax={deviceSummary.motor_frequency_max_hz}
                valueSerializer={frequencySerializer}
              >
                Frequency
              </GaugeLinear>
              <GaugeLinear
                value={deviceSummary.motor_speed_rpm}
                valueMin={0}
                valueMax={deviceSummary.motor_speed_rated_rpm}
                valueSerializer={rpmSerializer}
              >
                RPM
              </GaugeLinear>
              <GaugeLinearRatio value={deviceSummary.motor_torque}>Torque</GaugeLinearRatio>
              <GaugeLinearRatio value={deviceSummary.motor_power}>Power</GaugeLinearRatio>
            </SectionContent>
          </Section>
          <Section>
            <SectionTitle>Other</SectionTitle>
            <SectionContent>
              <GaugeLinear
                value={deviceSummary.dc_link_voltage_v}
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
export default Summary;

const Wrapper = styled.div``;

const Section = styled.div`
  margin-bottom: 0.5rem;
`;
const SectionTitle = styled.h3``;
const SectionContent = styled.div`
  padding-left: 1rem;
`;

function voltageSerializer(voltage: number): string {
  return `${voltage.toFixed(2)}V`;
}
function currentSerializer(voltage: number): string {
  return `${voltage.toFixed(2)}A`;
}
function frequencySerializer(voltage: number): string {
  return `${voltage.toFixed(2)}Hz`;
}
function rpmSerializer(voltage: number): string {
  return `${voltage.toFixed(0)}rpm`;
}
