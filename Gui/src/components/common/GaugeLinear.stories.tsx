import { Meta, Story } from "@storybook/react";
import GaugeLinear from "./GaugeLinear";

function valueSerializer(value: number): string {
  return `${(value * 100).toFixed(0)}%`;
}

export default {
  title: "components/common/GaugeLinear",
  argTypes: {
    value: {
      type: "number",
      defaultValue: 0.0,
      control: { type: "range", min: 0, max: 1, step: 0.01 },
    },
  },
} as Meta;

export const Primary: Story<{
  value: number;
}> = (props) => (
  <GaugeLinear valueMin={0.0} valueMax={1.0} valueSerializer={valueSerializer} {...props}>
    Description
  </GaugeLinear>
);
