import { WindowOpenStateOpenClosed, WindowOpenStateOpenTiltedClosed } from "@/datatypes/building/Window";
import { Meta } from "@storybook/react-vite";
import { WindowOpenStateOpenClosedComponent, WindowOpenStateOpenTiltedClosedComponent } from "./Window";

export default {
  title: "components/datatypes/building/Window",
} satisfies Meta;

export const Empty: React.FC = () => (
  <>
    <WindowOpenStateOpenClosedComponent value={undefined} />
    <WindowOpenStateOpenTiltedClosedComponent value={undefined} />
  </>
);
export const Basic: React.FC = () => (
  <>
    {Object.values(WindowOpenStateOpenClosed).map((value) => (
      <WindowOpenStateOpenClosedComponent key={value} value={value} />
    ))}
    {Object.values(WindowOpenStateOpenTiltedClosed).map((value) => (
      <WindowOpenStateOpenTiltedClosedComponent key={value} value={value} />
    ))}
  </>
);
