import { Meta } from "@storybook/react-vite";
import Component from "./Summary";

export default {
  title: "components/devices/soft/web/input/button_state_monostable_a/Summary",
  argTypes: {
    value: {
      type: "boolean",
      defaultValue: false,
    },
    onValueChanged: {
      action: "onValueChanged",
    },
  },
} satisfies Meta;

export const Basic: React.FC<{
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
