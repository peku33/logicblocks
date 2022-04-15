import { Meta, Story } from "@storybook/react";
import { useState } from "react";
import Component, { Data } from "./Summary";

export default {
  title: "components/devices/soft/web/button_event_boolean_a/Summary",
  argTypes: {
    value: {
      type: "boolean",
      defaultValue: false,
    },
    onPush: {
      action: "onPush",
    },
  },
} as Meta;

export const Managed: Story<{}> = () => {
  const [state, setState] = useState<Data | undefined>(false);
  return (
    <>
      <Component data={state} onPush={(newState) => setState(newState)} />
    </>
  );
};

export const Basic: Story<{
  value: boolean;
  onPush: () => void;
}> = (props) => {
  return (
    <>
      <Component data={props.value} onPush={props.onPush} />
    </>
  );
};

export const Empty: Story<{}> = () => {
  return (
    <>
      <Component data={undefined} onPush={() => ({})} />
    </>
  );
};
