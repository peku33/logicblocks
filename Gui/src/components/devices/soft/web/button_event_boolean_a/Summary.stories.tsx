import { Meta, Story } from "@storybook/react";
import Component from "./Summary";

export default {
  title: "components/devices/soft/web/button_event_boolean_a/Summary",
  argTypes: {
    onPush: {
      action: "onPush",
    },
  },
} as Meta;

export const Basic: Story<{
  onPush: () => void;
}> = (props) => {
  return (
    <>
      <Component onPush={props.onPush} />
    </>
  );
};
