import { Meta } from "@storybook/react";
import Component, { INPUT_COUNT } from "./SummaryInner";

export default {
  title: "components/devices/houseblocks/avr_v1/d0002_reed_switch_v1/SummaryInner",
} satisfies Meta;

export const Basic: React.FC = () => (
  <>
    <Component
      data={{
        inputs: Array.from(Array(INPUT_COUNT).keys()).map((index) => (index / INPUT_COUNT) * 40_000),
      }}
    />
  </>
);

export const Empty: React.FC = () => (
  <>
    <Component data={undefined} />
  </>
);
