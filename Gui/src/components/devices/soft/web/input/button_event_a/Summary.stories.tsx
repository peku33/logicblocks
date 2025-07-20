import { type Meta } from "@storybook/react-vite";
import Component from "./Summary";

export default {
  title: "components/devices/soft/web/input/button_event_a/Summary",
} satisfies Meta;

export const Basic: React.FC = () => {
  return (
    <>
      <Component onSignal={async () => {}} />
    </>
  );
};
