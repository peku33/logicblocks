import { type Meta } from "@storybook/react-vite";
import Component from "./Summary";

export default {
  title: "components/devices/soft/web/input/real_input_a/Summary",
  argTypes: {
    value: {
      type: "number",
      defaultValue: 0.0,
    },
    onValueChanged: {
      action: "onValueChanged",
    },
  },
} satisfies Meta;

export const Basic: React.FC<{
  value: number;
  onValueChanged: (newValue: number | null) => void;
}> = (props) => {
  const { value, onValueChanged } = props;

  return (
    <>
      <Component
        data={value}
        onValueChanged={async (newValue: number | null) => {
          onValueChanged(newValue);
        }}
      />
    </>
  );
};
