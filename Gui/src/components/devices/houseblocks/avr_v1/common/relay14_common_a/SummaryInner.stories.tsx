import { type Meta } from "@storybook/react-vite";
import Component, { OUTPUTS_COUNT } from "./SummaryInner";

export default {
  title: "components/devices/houseblocks/avr_v1/common/relay14_common_a/SummaryInner",
} satisfies Meta;

export const Basic: React.FC = () => (
  <>
    <Component
      data={{
        outputs: Array.from(Array(OUTPUTS_COUNT).keys()).map((index) => index % 2 === 0),
      }}
    />
  </>
);

export const Empty: React.FC = () => (
  <>
    <Component data={undefined} />
  </>
);
