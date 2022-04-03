import { Meta, Story } from "@storybook/react";
import { makeAvrV1Summary } from "./Summary";

export default {
  title: "components/devices/houseblocks/avr_v1/Summary",
} as Meta;

export const Basic: Story<{}> = () => (
  <>
    <DeviceComponentAvrV1
      data={{
        hardware_runner: { device_state: "Running" },
        device: { a: 7, b: "aaa" },
      }}
    />
  </>
);

export const Empty: Story<{}> = () => (
  <>
    <DeviceComponentAvrV1 data={undefined} />
  </>
);

const DeviceComponent: React.VFC<{
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
