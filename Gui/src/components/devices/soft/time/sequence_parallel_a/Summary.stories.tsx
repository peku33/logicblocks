import { type Meta } from "@storybook/react-vite";
import Component from "./Summary";

export default {
  title: "components/devices/soft/time/sequence_parallel_a/Summary",
} satisfies Meta;

export const Basic: React.FC = () => {
  return (
    <>
      <Component
        data={{
          configuration: {
            channels: [
              {
                name: "Channel 1",

                base_duration_seconds: 60.0,
                power_required: 0.5,

                round_min_seconds: 30.0,
                round_max_seconds: 180.0,
              },
              {
                name: "Channel 2",

                base_duration_seconds: 60.0,
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
        onDeviceDisable={async () => {}}
        onDevicePause={async () => {}}
        onDeviceEnable={async () => {}}
        onChannelsAllClear={async () => {}}
        onChannelsAllAdd={async () => {}}
        onChannelDisable={async () => {}}
        onChannelPause={async () => {}}
        onChannelEnable={async () => {}}
        onChannelClear={async () => {}}
        onChannelAdd={async () => {}}
        onChannelMoveFront={async () => {}}
        onChannelMoveBack={async () => {}}
      />
    </>
  );
};
