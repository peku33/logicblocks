import { Meta } from "@storybook/react-vite";
import Component from "./Summary";

export default {
  title: "components/devices/soft/web/button_event_a/Summary",
} satisfies Meta;

export const Basic: React.FC = () => {
  return (
    <>
      <Component onSignal={async () => {}} />
    </>
  );
};
