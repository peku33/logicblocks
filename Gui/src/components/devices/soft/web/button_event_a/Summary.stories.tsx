import { Meta, Story } from "@storybook/react";
import Component from "./Summary";

export default {
  title: "components/devices/soft/web/button_event_a/Summary",
} as Meta;

export const Basic: Story<{}> = () => {
  return (
    <>
      <Component onSignal={() => ({})} />
    </>
  );
};
