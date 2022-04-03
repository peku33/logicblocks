import { Meta, Story } from "@storybook/react";
import Component, { OUTPUT_COUNT } from "./SummaryInner";

export default {
  title: "components/devices/houseblocks/avr_v1/common/relay14_common_a/SummaryInner",
} as Meta;

export const Basic: Story<{}> = () => (
  <>
    <Component
      data={{
        values: [...Array(OUTPUT_COUNT)].map((_, index) => index % 2 === 0),
      }}
    />
  </>
);

export const Empty: Story<{}> = () => (
  <>
    <Component data={undefined} />
  </>
);
