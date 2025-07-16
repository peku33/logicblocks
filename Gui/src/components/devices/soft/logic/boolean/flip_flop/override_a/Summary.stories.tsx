import { Meta } from "@storybook/react-vite";
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
} satisfies Meta;

export const PassThrough: React.FC<{ inputValue: boolean }> = (props) => (
  <Component
    data={{
      input_value: props.inputValue,
      mode: {
        mode: "PassThrough",
      },
    }}
    onModeSet={async () => {}}
    onModeCyclePassThrough={async () => {}}
    onModeCycleNoPassThrough={async () => {}}
  />
);

export const Override: React.FC<{ inputValue: boolean; overrideValue: boolean }> = (props) => (
  <Component
    data={{
      input_value: props.inputValue,
      mode: {
        mode: "Override",
        value: props.overrideValue,
      },
    }}
    onModeSet={async () => {}}
    onModeCyclePassThrough={async () => {}}
    onModeCycleNoPassThrough={async () => {}}
  />
);

export const Empty: React.FC = () => (
  <Component
    data={undefined}
    onModeSet={async () => {}}
    onModeCyclePassThrough={async () => {}}
    onModeCycleNoPassThrough={async () => {}}
  />
);
