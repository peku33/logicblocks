import { Meta } from "@storybook/react-vite";
import Component from "./SummaryInner";

export default {
  title: "components/devices/houseblocks/avr_v1/d0003_junction_box_minimal_v1/SummaryInner",
} satisfies Meta;

export const Basic: React.FC = () => (
  <>
    <Component
      data={{
        temperature: 297.15,
      }}
    />
  </>
);

export const Empty: React.FC = () => (
  <>
    <Component data={undefined} />
  </>
);
