import { type Meta } from "@storybook/react-vite";
import Component from "./Summary";

export default {
  title: "components/devices/soft/time/up_down_a/Summary",
} satisfies Meta;

export const Empty: React.FC = () => <Component data={undefined} />;

export const Uncalibrated: React.FC = () => (
  <Component
    data={{
      state: {
        state: "Uncalibrated",
      },
    }}
  />
);

export const Calibrating: React.FC = () => (
  <Component
    data={{
      state: {
        state: "Calibrating",
        started_ago_seconds: 2.5,
        direction: "Up",
        duration_seconds: 10.0,
      },
    }}
  />
);

export const Stopped: React.FC = () => (
  <Component
    data={{
      state: {
        state: "Stopped",
        position: 0.75,
        uncertainty_relative: 0.25,
      },
    }}
  />
);

export const Moving: React.FC = () => (
  <Component
    data={{
      state: {
        state: "Moving",
        started_ago_seconds: 1.5,
        position_started: 0.25,
        position_target: 0.75,
        duration_seconds: 5.0,
        direction: "Up",
      },
    }}
  />
);
