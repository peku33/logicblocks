import { Meta, Story } from "@storybook/react";
import { ComponentProps } from "react";
import Summary from "./Summary";

export default {
  component: Summary,
  title: "devices/soft/web/button_event_boolean_a/Summary",
  argTypes: {
    onPush: {
      action: "onPush",
    },
  },
} as Meta;

export const Default: Story<ComponentProps<typeof Summary>> = (props) => <Summary {...props} />;
