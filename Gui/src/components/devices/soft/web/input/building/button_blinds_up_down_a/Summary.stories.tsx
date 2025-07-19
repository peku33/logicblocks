import { Meta } from "@storybook/react-vite";
import Component from "./Summary";

export default {
  title: "components/devices/soft/web/input/building/button_blinds_up_down_a",
  argTypes: {
    value: {
      type: "boolean",
      defaultValue: null,
    },
    onValueChanged: {
      action: "onValueChanged",
    },
  },
} satisfies Meta;

export const Basic: React.FC<{
  value: boolean | null;
  onValueChanged: (value: boolean | null) => void;
}> = (props) => {
  const { value, onValueChanged } = props;
  return (
    <>
      <Component data={value} onValueChanged={onValueChanged} />
    </>
  );
};
