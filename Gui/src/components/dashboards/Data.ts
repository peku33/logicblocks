import { Icon } from "@/components/common/FontAwesome";
import { DeviceId } from "@/components/devices/Device";
import { getJson } from "@/lib/Api";
import { useState } from "react";
import useAsyncEffect from "use-async-effect";
import * as Data from "./Data";

export interface Navigation {
  dashboards: DashboardSummary[];
}

export interface DashboardSummary {
  name: string;
  icon: Icon;
}

export interface DashboardContent {
  content: Content;
}

export interface ContentSectionContent {
  type: "SectionContent";

  section_content: SectionContent;
}
export interface ContentSections {
  type: "Sections";

  sections: Section[];
}
export type Content = ContentSectionContent | ContentSections;
export function contentIsSectionContent(content: Content): content is ContentSectionContent {
  return content.type === "SectionContent";
}
export function contentIsSections(content: Content): content is ContentSections {
  return content.type === "Sections";
}

export interface Section {
  name: string | null;

  content: SectionContent;
}

export interface SectionContentDashboards {
  type: "Dashboards";

  dashboards: DashboardSummary[];
}
export interface SectionContentDevices {
  type: "Devices";

  device_ids: DeviceId[];
}
export type SectionContent = SectionContentDashboards | SectionContentDevices;
export function sectionContentIsDashboards(sectionContent: SectionContent): sectionContent is SectionContentDashboards {
  return sectionContent.type === "Dashboards";
}
export function sectionContentIsDevices(sectionContent: SectionContent): sectionContent is SectionContentDevices {
  return sectionContent.type === "Devices";
}

export class ContentPathItemDashboard {
  public constructor(public readonly dashboardIndex: number) {}
}
export class ContentPathItemSectionDashboard {
  public constructor(
    public readonly sectionIndex: number,
    public readonly dashboardIndex: number,
  ) {}
}
export type ContentPathItem = ContentPathItemDashboard | ContentPathItemSectionDashboard;
export type ContentPath = ContentPathItem[];

function urlBuild(contentPath: ContentPath, endpoint: string): string {
  function contentPathItemToUrl(contentPathItem: ContentPathItem): string {
    if (contentPathItem instanceof ContentPathItemDashboard) {
      return `${contentPathItem.dashboardIndex}`;
    } else if (contentPathItem instanceof ContentPathItemSectionDashboard) {
      return `${contentPathItem.sectionIndex}:${contentPathItem.dashboardIndex}`;
    } else {
      throw new Error("unknown contentPathItem type");
    }
  }

  return `/gui/dashboards${contentPath
    .map((contentPathItem) => `/${contentPathItemToUrl(contentPathItem)}`)
    .join("")}${endpoint}`;
}

export function useNavigation(contentPath: Data.ContentPath): Data.Navigation | undefined {
  const [navigation, setNavigation] = useState<Data.Navigation | undefined>(undefined);

  useAsyncEffect(
    async (isMounted) => {
      const navigation = await fetchNavigation(contentPath);
      if (!isMounted()) return;
      setNavigation(navigation);
    },
    [...contentPath],
  );

  return navigation;
}
export async function fetchNavigation(contentPath: Data.ContentPath): Promise<Data.Navigation> {
  const dashboardContentPaths = Array.from(Array(contentPath.length + 1).keys()).map((index) =>
    contentPath.slice(0, index),
  );
  const dashboards = await Promise.all(
    dashboardContentPaths.map((dashboardContentPath) => fetchDashboardSummary(dashboardContentPath)),
  );
  const navigation: Data.Navigation = { dashboards };
  return navigation;
}
export async function fetchDashboardSummary(contentPath: Data.ContentPath): Promise<Data.DashboardSummary> {
  return await getJson<Data.DashboardSummary>(urlBuild(contentPath, "/summary"));
}

export function useDashboardContent(contentPath: Data.ContentPath): Data.DashboardContent | undefined {
  const [dashboardContent, setDashboardContent] = useState<Data.DashboardContent | undefined>(undefined);

  useAsyncEffect(
    async (isMounted) => {
      const dashboardContent = await fetchDashboardContent(contentPath);
      if (!isMounted()) return;
      setDashboardContent(dashboardContent);
    },
    [...contentPath],
  );

  return dashboardContent;
}
export async function fetchDashboardContent(contentPath: Data.ContentPath): Promise<Data.DashboardContent> {
  return await getJson<Data.DashboardContent>(urlBuild(contentPath, "/content"));
}
