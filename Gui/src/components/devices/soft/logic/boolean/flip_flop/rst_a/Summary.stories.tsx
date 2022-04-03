import { Meta, Story } from "@storybook/react";
import { useState } from "react";
import Component, { Data } from "./Summary";

export default {
  title: "components/devices/soft/logic/boolean/flip_flop/rst_a/Summary",
} as Meta;

export const Managed: Story<{}> = () => {
  const [state, setState] = useState<Data | undefined>(false);
  return (
    <>
      <Component data={state} onR={() => setState(false)} onS={() => setState(true)} onT={() => setState(undefined)} />
    </>
  );
};

export const Empty: Story<{}> = () => (
  <>
    <Component data={undefined} onR={() => ({})} onS={() => ({})} onT={() => ({})} />
  </>
);
