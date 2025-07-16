import { Meta } from "@storybook/react-vite";
import { useState } from "react";
import Component, { Data } from "./Summary";

export default {
  title: "components/devices/soft/logic/boolean/flip_flop/rst_a/Summary",
} satisfies Meta;

export const Managed: React.FC = () => {
  const [state, setState] = useState<Data | undefined>(false);
  return (
    <>
      <Component
        data={state}
        onR={async () => {
          setState(false);
        }}
        onS={async () => {
          setState(true);
        }}
        onT={async () => {
          setState(undefined);
        }}
      />
    </>
  );
};

export const Empty: React.FC = () => (
  <>
    <Component data={undefined} onR={async () => {}} onS={async () => {}} onT={async () => {}} />
  </>
);
