import { Meta, Story } from "@storybook/react";
import Component from "./Summary";

export default {
  title: "components/devices/soft/web/button_state_monostable_a/Summary",
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

export const Basic: Story<{
  value: boolean;
  onValueChanged: (value: boolean) => void;
}> = (props) => {
  const { value, onValueChanged } = props;
  return (
    <>
      <Component data={value} onValueChanged={onValueChanged} />
    </>
  );
};
