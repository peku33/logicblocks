import { Meta, Story } from "@storybook/react";
import Component from "./SummaryInner";

export default {
  title: "components/devices/houseblocks/avr_v1/d0003_junction_box_minimal_v1/SummaryInner",
} as Meta;

export const Basic: Story<{}> = () => (
  <>
    <Component
      data={{
        temperature: 297.15,
      }}
    />
  </>
);

export const Empty: Story<{}> = () => (
  <>
    <Component data={undefined} />
  </>
);
