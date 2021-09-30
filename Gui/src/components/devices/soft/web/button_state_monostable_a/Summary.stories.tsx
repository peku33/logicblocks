import { Meta, Story } from "@storybook/react";
import { ComponentProps } from "react";
import Summary from "./Summary";

export default {
  component: Summary,
  title: "devices/soft/web/button_state_monostable_a/Summary",
  argTypes: {
    value: {
      type: "boolean",
      defaultValue: false,
    },
    onValueChanged: {
      action: "onValueChanged",
    },
  },
} as Meta;

export const Default: Story<ComponentProps<typeof Summary>> = (props) => <Summary {...props} />;
