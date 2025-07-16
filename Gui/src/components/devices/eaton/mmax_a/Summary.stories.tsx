import { Meta } from "@storybook/react-vite";
import Component from "./Summary";

export default {
  title: "components/devices/eaton/mmax_a/Summary",
} satisfies Meta;

export const Running: React.FC = () => (
  <Component
    data={{
      state: "Running",

      warning: 7,

      speed_control_active: false,

      ready: true,
      running: false,
      speed_setpoint: 0.45,
      speed_actual: 0.55,
      reverse: false,

      motor_voltage_max_v: 230.0,
      motor_current_rated_a: 10.0,
      motor_current_max_a: 12.0,
      motor_frequency_min_hz: 10.0,
      motor_frequency_max_hz: 60.0,
      motor_frequency_rated_hz: 60.0,
      motor_speed_rated_rpm: 1440.0,

      motor_voltage_v: 200.0,
      motor_current_a: 8.0,
      motor_frequency_hz: 5.0,
      motor_speed_rpm: 1000.0,
      motor_torque: 0.5,
      motor_power: 0.95,

      dc_link_voltage_v: 320.1,
      remote_input: true,
    }}
  />
);

export const Error: React.FC = () => <Component data={{ state: "Error" }} />;
export const Initializing: React.FC = () => <Component data={{ state: "Initializing" }} />;

export const Empty: React.FC = () => <Component data={undefined} />;
