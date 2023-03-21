import { Meta, Story } from "@storybook/react";
import Component from "./Summary";

export default {
  title: "components/devices/soft/logic/boolean/flip_flop/override_a/Summary",
  argTypes: {
    inputValue: {
      type: "boolean",
      defaultValue: false,
    },
    overrideValue: {
      type: "boolean",
      defaultValue: false,
    },
  },
} as Meta;

export const PassThrough: Story<{ inputValue: boolean }> = (props) => (
  <Component
    data={{
      input_value: props.inputValue,
      mode: {
        mode: "PassThrough",
      },
    }}
    onModeSet={() => ({})}
    onModeCyclePassThrough={() => ({})}
    onModeCycleNoPassThrough={() => ({})}
  />
);

export const Override: Story<{ inputValue: boolean; overrideValue: boolean }> = (props) => (
  <Component
    data={{
      input_value: props.inputValue,
      mode: {
        mode: "Override",
        value: props.overrideValue,
      },
    }}
    onModeSet={() => ({})}
    onModeCyclePassThrough={() => ({})}
    onModeCycleNoPassThrough={() => ({})}
  />
);

export const Empty: Story<{}> = (props) => (
  <Component
    data={undefined}
    onModeSet={() => ({})}
    onModeCyclePassThrough={() => ({})}
    onModeCycleNoPassThrough={() => ({})}
  />
);
