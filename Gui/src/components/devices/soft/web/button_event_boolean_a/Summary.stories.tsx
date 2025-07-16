import { Meta } from "@storybook/react-vite";
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
} satisfies Meta;

export const Managed: React.FC = () => {
  const [state, setState] = useState<Data | undefined>(false);
  return (
    <>
      <Component
        data={state}
        onPush={async (newState) => {
          setState(newState);
        }}
      />
    </>
  );
};

export const Basic: React.FC<{
  value: boolean;
  onPush: () => void;
}> = (props) => {
  return (
    <>
      <Component
        data={props.value}
        onPush={async () => {
          props.onPush();
        }}
      />
    </>
  );
};

export const Empty: React.FC = () => {
  return (
    <>
      <Component data={undefined} onPush={async () => {}} />
    </>
  );
};
