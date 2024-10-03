import { Meta } from "@storybook/react";
import Component, { Data } from "./Summary";

export default {
  title: "components/devices/soft/web/display/boolean_a/Summary",
  argTypes: {
    value: {
      type: "boolean",
      defaultValue: false,
    },
  },
} satisfies Meta;

export const Basic: React.FC<{
  value: Data;
}> = (props) => {
  const { value } = props;

  return (
    <>
      <Component data={value} />
    </>
  );
};

export const Empty: React.FC = () => (
  <>
    <Component data={undefined} />
  </>
);
