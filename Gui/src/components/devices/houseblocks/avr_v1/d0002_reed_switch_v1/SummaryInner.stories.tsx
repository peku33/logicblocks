import { type Meta } from "@storybook/react-vite";
import Component, { INPUTS_COUNT } from "./SummaryInner";

export default {
  title: "components/devices/houseblocks/avr_v1/d0002_reed_switch_v1/SummaryInner",
} satisfies Meta;

export const Basic: React.FC = () => (
  <>
    <Component
      data={{
        inputs: Array.from(Array(INPUTS_COUNT).keys()).map((index) => (index / INPUTS_COUNT) * 40_000),
      }}
    />
  </>
);

export const Empty: React.FC = () => (
  <>
    <Component data={undefined} />
  </>
);
