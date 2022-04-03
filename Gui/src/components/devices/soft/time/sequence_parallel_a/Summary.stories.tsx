import { Meta, Story } from "@storybook/react";
import Component from "./Summary";

export default {
  title: "components/devices/soft/time/sequence_parallel_a/Summary",
} as Meta;

export const Basic: Story<{}> = () => {
  return (
    <>
      <Component
        data={{
          configuration: {
            channels: [
              {
                name: "Channel 1",

                base_time_seconds: 60.0,
                power_required: 0.5,

                round_min_seconds: 30.0,
                round_max_seconds: 180.0,
              },
              {
                name: "Channel 2",

                base_time_seconds: 60.0,
                power_required: 0.75,

                round_min_seconds: 45.0,
                round_max_seconds: 300.0,
              },
            ],
            power_max: 1.0,
          },
          state: {
            state: "Enabled",
            power: 0.75,
            channels: [
              {
                state: "EnabledQueued",
                queue_seconds: 120.0,
                queue_position: 1,
              },
              {
                state: "EnabledActive",
                queue_seconds: 60.0,
                round_seconds: 50.0,
              },
            ],
          },
        }}
        onDeviceDisable={() => ({})}
        onDevicePause={() => ({})}
        onDeviceEnable={() => ({})}
        onChannelsAllClear={() => ({})}
        onChannelsAllAdd={() => ({})}
        onChannelDisable={() => ({})}
        onChannelPause={() => ({})}
        onChannelEnable={() => ({})}
        onChannelClear={() => ({})}
        onChannelAdd={() => ({})}
        onChannelMoveFront={() => ({})}
        onChannelMoveBack={() => ({})}
      />
    </>
  );
};
