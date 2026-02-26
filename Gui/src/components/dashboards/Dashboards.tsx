import Colors from "@/components/common/Colors";
import FontAwesomeIcon from "@/components/common/FontAwesome";
import Loader from "@/components/common/Loader";
import { type DeviceId } from "@/components/devices/Device";
import { type PropsWithChildren } from "react";
import styled from "styled-components";
import * as Data from "./Data";

export type NavigationLinkComponent = React.FC<PropsWithChildren>;
export type NavigationLinkComponentResolver = (backDepth: number) => NavigationLinkComponent;

export type DashboardLinkComponent = React.FC<PropsWithChildren>;
export type DashboardLinkComponentResolver = (childContentPathItem: Data.ContentPathItem) => DashboardLinkComponent;

export type DeviceListComponent = React.FC<{ deviceIds: DeviceId[] }>;

// combined
export const Page: React.FC<{
  navigation: Data.Navigation | undefined;
  navigationLinkComponentResolver: NavigationLinkComponentResolver;
  dashboardContent: Data.DashboardContent | undefined;
  dashboardLinkComponentResolver: DashboardLinkComponentResolver;
  deviceListComponent: DeviceListComponent;
}> = (props) => {
  const {
    navigation,
    navigationLinkComponentResolver,
    dashboardContent,
    dashboardLinkComponentResolver,
    deviceListComponent,
  } = props;

  return (
    <PageWrapper>
      <PageNavigationWrapper>
        <Navigation navigation={navigation} navigationLinkComponentResolver={navigationLinkComponentResolver} />
      </PageNavigationWrapper>
      <PageDashboardContentWrapper>
        <DashboardContent
          dashboardContent={dashboardContent}
          dashboardLinkComponentResolver={dashboardLinkComponentResolver}
          deviceListComponent={deviceListComponent}
        />
      </PageDashboardContentWrapper>
    </PageWrapper>
  );
};
const PageWrapper = styled.div``;
const PageNavigationWrapper = styled.div`
  padding: 0.25rem;
  margin-bottom: 0.25rem;
  border-bottom: solid 1px ${Colors.GREY};
`;
const PageDashboardContentWrapper = styled.div`
  padding: 0.25rem;
`;

// navigation
export const Navigation: React.FC<{
  navigation: Data.Navigation | undefined;
  navigationLinkComponentResolver: NavigationLinkComponentResolver;
}> = (props) => {
  const { navigation, navigationLinkComponentResolver } = props;

  if (navigation === undefined) {
    return <Loader sizeRem={1} />;
  }

  return (
    <NavigationList>
      {navigation.dashboards.map((dashboard, dashboardIndex) => (
        <NavigationListItem key={dashboardIndex}>
          {dashboardIndex !== 0 && (
            <NavigationListItemSeparatorWrapper>
              <FontAwesomeIcon icon={{ prefix: "fas", name: "chevron-right" }} />
            </NavigationListItemSeparatorWrapper>
          )}
          <NavigationItem
            dashboard={dashboard}
            navigationLinkComponent={navigationLinkComponentResolver(navigation.dashboards.length - dashboardIndex - 1)}
          />
        </NavigationListItem>
      ))}
    </NavigationList>
  );
};
const NavigationList = styled.div`
  display: flex;
  align-items: center;
`;
const NavigationListItem = styled.div`
  display: flex;
  align-items: center;
`;
const NavigationListItemSeparatorWrapper = styled.div`
  margin: 0 0.5rem;
`;

const NavigationItem: React.FC<{
  dashboard: Data.DashboardSummary;
  navigationLinkComponent: NavigationLinkComponent;
}> = (props) => {
  const { dashboard, navigationLinkComponent } = props;

  const NavigationLinkComponent = navigationLinkComponent;
  return (
    <NavigationLinkComponent>
      <NavigationItemWrapper>
        <NavigationItemIconWrapper>
          <FontAwesomeIcon icon={dashboard.icon} />
        </NavigationItemIconWrapper>
        <NavigationItemName>{dashboard.name}</NavigationItemName>
      </NavigationItemWrapper>
    </NavigationLinkComponent>
  );
};
const NavigationItemWrapper = styled.div`
  display: flex;
  align-items: center;
`;
const NavigationItemIconWrapper = styled.div`
  margin-right: 0.25rem;
`;
const NavigationItemName = styled.div``;

// content
export const DashboardContent: React.FC<{
  dashboardContent: Data.DashboardContent | undefined;
  dashboardLinkComponentResolver: DashboardLinkComponentResolver;
  deviceListComponent: DeviceListComponent;
}> = (props) => {
  const { dashboardContent, dashboardLinkComponentResolver, deviceListComponent } = props;

  if (dashboardContent === undefined) {
    return <Loader sizeRem={4} />;
  }

  return (
    <Content
      content={dashboardContent.content}
      dashboardLinkComponentResolver={dashboardLinkComponentResolver}
      deviceListComponent={deviceListComponent}
    />
  );
};

const Content: React.FC<{
  content: Data.Content;
  dashboardLinkComponentResolver: DashboardLinkComponentResolver;
  deviceListComponent: DeviceListComponent;
}> = (props) => {
  const { content, dashboardLinkComponentResolver, deviceListComponent } = props;

  if (Data.contentIsSectionContent(content)) {
    return (
      <ContentSectionContent
        contentSectionContent={content}
        dashboardLinkComponentResolver={dashboardLinkComponentResolver}
        deviceListComponent={deviceListComponent}
      />
    );
  } else if (Data.contentIsSections(content)) {
    return (
      <ContentSections
        contentSections={content}
        dashboardLinkComponentResolver={dashboardLinkComponentResolver}
        deviceListComponent={deviceListComponent}
      />
    );
  } else {
    throw new Error("unknown content type");
  }
};

const ContentSectionContent: React.FC<{
  contentSectionContent: Data.ContentSectionContent;
  dashboardLinkComponentResolver: DashboardLinkComponentResolver;
  deviceListComponent: DeviceListComponent;
}> = (props) => {
  const { contentSectionContent, dashboardLinkComponentResolver, deviceListComponent } = props;

  return (
    <SectionContent
      sectionContent={contentSectionContent.section_content}
      sectionIndex={undefined}
      dashboardLinkComponentResolver={dashboardLinkComponentResolver}
      deviceListComponent={deviceListComponent}
    />
  );
};

const ContentSections: React.FC<{
  contentSections: Data.ContentSections;
  dashboardLinkComponentResolver: DashboardLinkComponentResolver;
  deviceListComponent: DeviceListComponent;
}> = (props) => {
  const { contentSections, dashboardLinkComponentResolver, deviceListComponent } = props;

  return (
    <ContentSectionsWrapper>
      {contentSections.sections.map((section, sectionIndex) => (
        <ContentSectionsItemWrapper key={sectionIndex}>
          <Section
            section={section}
            sectionIndex={sectionIndex}
            dashboardLinkComponentResolver={dashboardLinkComponentResolver}
            deviceListComponent={deviceListComponent}
          />
        </ContentSectionsItemWrapper>
      ))}
    </ContentSectionsWrapper>
  );
};
const ContentSectionsWrapper = styled.div``;
const ContentSectionsItemWrapper = styled.div``;

const Section: React.FC<{
  section: Data.Section;
  sectionIndex: number;
  dashboardLinkComponentResolver: DashboardLinkComponentResolver;
  deviceListComponent: DeviceListComponent;
}> = (props) => {
  const { section, sectionIndex, dashboardLinkComponentResolver, deviceListComponent } = props;

  return (
    <SectionWrapper>
      {section.name !== null && <SectionName>{section.name}</SectionName>}
      <SectionContentWrapper>
        <SectionContent
          sectionContent={section.content}
          sectionIndex={sectionIndex}
          dashboardLinkComponentResolver={dashboardLinkComponentResolver}
          deviceListComponent={deviceListComponent}
        />
      </SectionContentWrapper>
    </SectionWrapper>
  );
};
const SectionWrapper = styled.div``;
const SectionName = styled.h4`
  margin-top: 0.25rem;

  font-weight: bold;
  font-variant: small-caps;
`;
const SectionContentWrapper = styled.div`
  border-top: solid 1px ${Colors.GREY_DARK};
  padding: 0.25rem 0;
`;

const SectionContent: React.FC<{
  sectionContent: Data.SectionContent;
  sectionIndex: number | undefined;
  dashboardLinkComponentResolver: DashboardLinkComponentResolver;
  deviceListComponent: DeviceListComponent;
}> = (props) => {
  const { sectionContent, sectionIndex, dashboardLinkComponentResolver, deviceListComponent } = props;

  if (Data.sectionContentIsDashboards(sectionContent)) {
    return (
      <SectionContentDashboards
        sectionContentDashboards={sectionContent}
        sectionIndex={sectionIndex}
        dashboardLinkComponentResolver={dashboardLinkComponentResolver}
      />
    );
  } else if (Data.sectionContentIsDevices(sectionContent)) {
    return <SectionContentDevices sectionContentDevices={sectionContent} deviceListComponent={deviceListComponent} />;
  } else {
    throw new Error("unknown sectionContent type");
  }
};

const SectionContentDashboards: React.FC<{
  sectionContentDashboards: Data.SectionContentDashboards;
  sectionIndex: number | undefined;
  dashboardLinkComponentResolver: DashboardLinkComponentResolver;
}> = (props) => {
  const { sectionContentDashboards, sectionIndex, dashboardLinkComponentResolver } = props;

  return (
    <SectionContentDashboardsList>
      {sectionContentDashboards.dashboards.map((sectionContentDashboard, dashboardIndex) => (
        <SectionContentDashboardsListItem key={dashboardIndex}>
          <SectionContentDashboard
            sectionContentDashboard={sectionContentDashboard}
            sectionIndex={sectionIndex}
            dashboardIndex={dashboardIndex}
            dashboardLinkComponentResolver={dashboardLinkComponentResolver}
          />
        </SectionContentDashboardsListItem>
      ))}
    </SectionContentDashboardsList>
  );
};
const SectionContentDashboardsList = styled.div`
  display: grid;
  grid-gap: 0.25rem;

  grid-template-columns: repeat(auto-fit, minmax(300px, auto));
  grid-auto-rows: 1fr;

  align-items: center;
  justify-content: center;
`;
const SectionContentDashboardsListItem = styled.div``;

const SectionContentDashboard: React.FC<{
  sectionContentDashboard: Data.DashboardSummary;
  sectionIndex: number | undefined;
  dashboardIndex: number;
  dashboardLinkComponentResolver: DashboardLinkComponentResolver;
}> = (props) => {
  const { sectionContentDashboard, sectionIndex, dashboardIndex, dashboardLinkComponentResolver } = props;

  const childContentPathItem: Data.ContentPathItem =
    sectionIndex !== undefined
      ? new Data.ContentPathItemSectionDashboard(sectionIndex, dashboardIndex)
      : new Data.ContentPathItemDashboard(dashboardIndex);
  const DashboardLinkComponent = dashboardLinkComponentResolver(childContentPathItem);

  return (
    /* eslint-disable-next-line react-hooks/static-components */
    <DashboardLinkComponent>
      <SectionContentDashboardWrapper>
        <SectionContentDashboardIconWrapper>
          <FontAwesomeIcon icon={sectionContentDashboard.icon} />
        </SectionContentDashboardIconWrapper>
        <SectionContentDashboardName>{sectionContentDashboard.name}</SectionContentDashboardName>
      </SectionContentDashboardWrapper>
    </DashboardLinkComponent>
  );
};
const SectionContentDashboardWrapper = styled.div`
  padding: 0.25rem;
  border: solid 1px ${Colors.GREY};

  text-align: center;
`;
const SectionContentDashboardIconWrapper = styled.div`
  margin: auto;

  font-size: 3rem;
  margin-bottom: 1rem;
`;
const SectionContentDashboardName = styled.h3`
  font-size: 2rem;
`;

const SectionContentDevices: React.FC<{
  sectionContentDevices: Data.SectionContentDevices;
  deviceListComponent: DeviceListComponent;
}> = (props) => {
  const { sectionContentDevices, deviceListComponent } = props;
  const DeviceListComponent = deviceListComponent;

  return <DeviceListComponent deviceIds={sectionContentDevices.device_ids} />;
};
