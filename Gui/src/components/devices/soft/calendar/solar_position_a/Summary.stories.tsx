import { type Meta } from "@storybook/react-vite";
import Component from "./Summary";

export default {
  title: "components/devices/soft/calendar/solar_position_a/Summary",
} satisfies Meta;

export const Basic: React.FC = () => (
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

export const Empty: React.FC = () => (
  <>
    <Component data={undefined} />
  </>
);
