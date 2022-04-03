import { Meta, Story } from "@storybook/react";
import Component from "./Summary";

export default {
  title: "components/devices/soft/web/ratio_slider_a/Summary",
  argTypes: {
    value: {
      type: "number",
      defaultValue: 0.0,
      control: { type: "range", min: 0, max: 1, step: 0.01 },
    },
    onValueChanged: {
      action: "onValueChanged",
    },
  },
} as Meta;

export const Basic: Story<{
  value: number;
  onValueChanged: (newValue: number | null) => void;
}> = (props) => {
  const { value, onValueChanged } = props;
  return (
    <>
      <Component data={value} onValueChanged={onValueChanged} />
    </>
  );
};
