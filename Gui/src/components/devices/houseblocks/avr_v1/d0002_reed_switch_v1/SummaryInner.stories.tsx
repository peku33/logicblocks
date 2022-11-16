import { Meta, Story } from "@storybook/react";
import Component, { INPUT_COUNT } from "./SummaryInner";

export default {
  title: "components/devices/houseblocks/avr_v1/d0002_reed_switch_v1/SummaryInner",
} as Meta;

export const Basic: Story<{}> = () => (
  <>
    <Component
      data={{
        inputs: Array.from(Array(INPUT_COUNT).keys()).map((index) => (index / INPUT_COUNT) * 40_000),
      }}
    />
  </>
);

export const Empty: Story<{}> = () => (
  <>
    <Component data={undefined} />
  </>
);
