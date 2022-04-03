import { Meta, Story } from "@storybook/react";
import Component from "./Summary";

export default {
  title: "components/devices/soft/calendar/solar_position_a/Summary",
} as Meta;

export const Basic: Story<{}> = () => (
  <>
    <Component
      data={{
        julian_day: 2459641.5,
        elevation: -1.51843645, // -87 deg
        asimuth: 4.71238898, // 270
      }}
    />
  </>
);

export const Empty: Story<{}> = () => (
  <>
    <Component data={undefined} />
  </>
);
