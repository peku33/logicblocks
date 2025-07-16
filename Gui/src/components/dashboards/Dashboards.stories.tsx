import { DeviceId } from "@/components/devices/Device";
import { Meta } from "@storybook/react-vite";

import {
  DashboardLinkComponent,
  DashboardLinkComponentResolver,
  NavigationLinkComponentResolver,
  Page,
} from "./Dashboards";
import * as Data from "./Data";

export default {
  title: "components/dashboards/Dashboards",
} satisfies Meta;

const navigationLinkComponentResolver: NavigationLinkComponentResolver = (backDepth: number) => {
  const NavigationLinkComponent: DashboardLinkComponent = (props) => {
    return (
      <a href="#" title={JSON.stringify(backDepth)}>
        {props.children}
      </a>
    );
  };
  return NavigationLinkComponent;
};

const dashboardLinkComponentResolver: DashboardLinkComponentResolver = (childContentPathItem: Data.ContentPathItem) => {
  const DashboardLinkComponent: DashboardLinkComponent = (props) => {
    return (
      <a href="#" title={JSON.stringify(childContentPathItem)}>
        {props.children}
      </a>
    );
  };
  return DashboardLinkComponent;
};

const DeviceListComponent: React.FC<{ deviceIds: DeviceId[] }> = (props) => {
  const { deviceIds } = props;

  return <pre>{JSON.stringify(deviceIds)}</pre>;
};

export const Empty: React.FC = () => (
  <Page
    navigation={undefined}
    navigationLinkComponentResolver={navigationLinkComponentResolver}
    dashboardContent={undefined}
    dashboardLinkComponentResolver={dashboardLinkComponentResolver}
    deviceListComponent={DeviceListComponent}
  />
);

export const ContentSectionContent: React.FC = () => (
  <Page
    navigation={{
      dashboards: [
        { name: "Home", icon: { prefix: "fas", name: "home" } },
        { name: "Step 1", icon: { prefix: "fas", name: "brush" } },
        { name: "Step 2", icon: { prefix: "fas", name: "person" } },
      ],
    }}
    navigationLinkComponentResolver={navigationLinkComponentResolver}
    dashboardContent={{
      content: {
        type: "SectionContent",
        section_content: {
          type: "Dashboards",
          dashboards: [
            { name: "Adin", icon: { prefix: "fas", name: "1" } },
            { name: "Dwa", icon: { prefix: "fas", name: "2" } },
            { name: "Tri", icon: { prefix: "fas", name: "3" } },
          ],
        },
      },
    }}
    dashboardLinkComponentResolver={dashboardLinkComponentResolver}
    deviceListComponent={DeviceListComponent}
  />
);
export const ContentSections: React.FC = () => (
  <Page
    navigation={{
      dashboards: [{ name: "Home", icon: { prefix: "fas", name: "home" } }],
    }}
    navigationLinkComponentResolver={navigationLinkComponentResolver}
    dashboardContent={{
      content: {
        type: "Sections",
        sections: [
          {
            name: "Section1",
            content: {
              type: "Dashboards",
              dashboards: [
                {
                  name: "Section 1, Dashboard 1",
                  icon: {
                    prefix: "fas",
                    name: "mask",
                  },
                },
                {
                  name: "Section 1, Dashboard 2",
                  icon: {
                    prefix: "fas",
                    name: "house",
                  },
                },
              ],
            },
          },
          {
            name: null,
            content: {
              type: "Dashboards",
              dashboards: Array.from(Array(10).keys()).map((index) => ({
                name: `Section 2, Dashboard ${index}`,
                icon: {
                  prefix: "fas",
                  name: "gear",
                },
              })),
            },
          },
          {
            name: "Section3",
            content: {
              type: "Devices",
              device_ids: [1, 2, 3, 4],
            },
          },
          {
            name: "Section4",
            content: {
              type: "Devices",
              device_ids: [5, 6, 7, 8],
            },
          },
        ],
      },
    }}
    dashboardLinkComponentResolver={dashboardLinkComponentResolver}
    deviceListComponent={DeviceListComponent}
  />
);
