import { Meta, Story } from "@storybook/react";
import Component from "./Summary";

export default {
  title: "components/devices/hikvision/ds2cd2x32x_x/Summary",
} as Meta;

export const Basic: Story<{}> = () => (
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
      snapshotBaseUrl={undefined}
    />
  </>
);

export const Empty: Story<{}> = () => (
  <>
    <Component data={undefined} snapshotBaseUrl={undefined} />
  </>
);
