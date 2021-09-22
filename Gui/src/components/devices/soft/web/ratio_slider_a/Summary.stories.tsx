import { Meta, Story } from "@storybook/react";
import { ComponentProps } from "react";
import Summary from "./Summary";

export default {
  component: Summary,
  title: "Devices/Soft/Web/RatioSliderA/Summary",
  argTypes: {
    value: {
      type: "number",
      defaultValue: 0.0,
      control: { type: "range", min: 0, max: 1, step: 0.01 },
    },
    valueChanged: {
      action: "valueChanged",
    },
  },
} as Meta;

export const Template: Story<ComponentProps<typeof Summary>> = (props) => <Summary {...props} />;
