import { Meta, Story } from "@storybook/react";
import { ComponentProps } from "react";
import Summary from "./Summary";

export default {
  component: Summary,
  title: "devices/soft/web/ratio_slider_a/Summary",
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

export const Default: Story<ComponentProps<typeof Summary>> = (props) => <Summary {...props} />;
