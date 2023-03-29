import { Meta, Story } from "@storybook/react";
import Component, { Data } from "./Summary";

export default {
  title: "components/devices/soft/web/display/boolean_a/Summary",
  argTypes: {
    value: {
      type: "boolean",
      defaultValue: false,
    },
  },
} as Meta;

export const Basic: Story<{
  value: Data;
}> = (props) => {
  const { value } = props;

  return (
    <>
      <Component data={value} />
    </>
  );
};

export const Empty: Story<{}> = () => (
  <>
    <Component data={undefined} />
  </>
);
