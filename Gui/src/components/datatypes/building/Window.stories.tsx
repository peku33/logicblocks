import { Meta, Story } from "@storybook/react";
import { WindowOpenStateOpenClosed, WindowOpenStateOpenTiltedClosed } from "datatypes/building/Window";
import { WindowOpenStateOpenClosedComponent, WindowOpenStateOpenTiltedClosedComponent } from "./Window";

export default {
  title: "components/datatypes/building/Window",
} as Meta;

export const Empty: Story<{}> = () => (
  <>
    <WindowOpenStateOpenClosedComponent value={undefined} />
    <WindowOpenStateOpenTiltedClosedComponent value={undefined} />
  </>
);
export const Basic: Story<{}> = () => (
  <>
    {Object.values(WindowOpenStateOpenClosed).map((value) => (
      <WindowOpenStateOpenClosedComponent key={value} value={value} />
    ))}
    {Object.values(WindowOpenStateOpenTiltedClosed).map((value) => (
      <WindowOpenStateOpenTiltedClosedComponent key={value} value={value} />
    ))}
  </>
);
