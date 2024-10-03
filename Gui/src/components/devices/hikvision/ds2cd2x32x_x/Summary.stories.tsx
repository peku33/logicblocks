import { Meta } from "@storybook/react";
import Component from "./Summary";

export default {
  title: "components/devices/hikvision/ds2cd2x32x_x/Summary",
} satisfies Meta;

export const Basic: React.FC = () => (
  <>
    <Component
      data={{
        state: "Running",
        snapshot_updated: null,
        rtsp_urls: {
          main: "main",
          sub: "sub1",
        },
        events: {
          camera_failure: true,
          video_loss: false,
          tampering_detection: true,
          motion_detection: false,
          line_detection: true,
          field_detection: false,
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
