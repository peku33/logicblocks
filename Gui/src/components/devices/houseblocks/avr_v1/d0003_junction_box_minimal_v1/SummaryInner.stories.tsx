import { type Meta } from "@storybook/react-vite";
import Component, { KEYS_COUNT, LEDS_COUNT } from "./SummaryInner";

export default {
  title: "components/devices/houseblocks/avr_v1/d0003_junction_box_minimal_v1/SummaryInner",
} satisfies Meta;

export const Basic: React.FC = () => (
  <>
    <Component
      data={{
        keys: Array.from(Array(KEYS_COUNT).keys()).map((index) => index % 2 === 0),
        leds: Array.from(Array(LEDS_COUNT).keys()).map((index) => index % 2 === 1),
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
