import { type Meta } from "@storybook/react-vite";
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

      motor_voltage_max: 230.0,
      motor_current_rated: 10.0,
      motor_current_max: 12.0,
      motor_frequency_min: 10.0,
      motor_frequency_max: 60.0,
      motor_frequency_rated: 60.0,
      motor_speed_rated: 1440.0 / 60.0,

      motor_voltage: 200.0,
      motor_current: 8.0,
      motor_frequency: 5.0,
      motor_speed: 1000.0 / 60.0,
      motor_torque: 0.5,
      motor_power: 0.95,

      dc_link_voltage: 320.1,
      remote_input: true,
    }}
  />
);

export const Error: React.FC = () => <Component data={{ state: "Error" }} />;
export const Initializing: React.FC = () => <Component data={{ state: "Initializing" }} />;

export const Empty: React.FC = () => <Component data={undefined} />;
