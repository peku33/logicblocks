import { type Meta } from "@storybook/react-vite";

import { makeAvrV1Summary } from "./Summary";

export default {
  title: "components/devices/houseblocks/avr_v1/Summary",
} satisfies Meta;

export const Basic: React.FC = () => (
  <>
    <DeviceComponentAvrV1
      data={{
        hardware_runner: { device_state: "Running" },
        device: { a: 7, b: "aaa" },
      }}
    />
  </>
);

export const Empty: React.FC = () => (
  <>
    <DeviceComponentAvrV1 data={undefined} />
  </>
);

const DeviceComponent: React.FC<{
  data:
    | {
        a: number;
        b: string;
      }
    | undefined;
}> = (props) => {
  const { data } = props;
  const { a, b } = data || { a: 0, b: "unknown" };

  return (
    <>
      <p>a: {a}</p>
      <p>b: {b}</p>
    </>
  );
};
const DeviceComponentAvrV1 = makeAvrV1Summary(DeviceComponent);
