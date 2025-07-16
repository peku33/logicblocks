import { Meta } from "@storybook/react-vite";
import Component from "./Summary";

export default {
  title: "components/devices/dahua/ipc_a/Summary",
} satisfies Meta;

export const Basic: React.FC = () => (
  <>
    <Component
      data={{
        state: "Running",
        snapshot_updated: null,
        rtsp_urls: {
          main: "main",
          sub1: "sub1",
          sub2: "sub2",
        },
        events: {
          video_blind: true,
          scene_change: false,
          video_motion: true,
          audio_mutation: false,
          smart_motion_human: true,
          smart_motion_vehicle: false,
        },
      }}
      snapshotEndpoint={undefined}
    />
  </>
);

export const Empty: React.FC = () => (
  <>
    <Component data={undefined} snapshotEndpoint={undefined} />
  </>
);
