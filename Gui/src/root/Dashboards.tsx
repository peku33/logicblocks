import {
  type DashboardLinkComponent,
  type DashboardLinkComponentResolver,
  type NavigationLinkComponent,
  type NavigationLinkComponentResolver,
} from "@/components/dashboards/Dashboards";
import { PageManaged } from "@/components/dashboards/DashboardsManaged";
import * as Data from "@/components/dashboards/Data";
import { useCallback } from "react";
import { Link, Route, Routes, useParams } from "react-router";
import Error404 from "./Error404";

const Root: React.FC = () => {
  return <Content contentPath={[]} />;
};
export default Root;

const Content: React.FC<{ contentPath: Data.ContentPath }> = (props) => {
  const { contentPath } = props;

  const navigationLinkComponentResolver: NavigationLinkComponentResolver = useCallback((backDepth: number) => {
    const NavigationLinkComponent: NavigationLinkComponent = (props) => {
      const { children } = props;
      return <Link to={`.${"/..".repeat(backDepth)}`}>{children}</Link>;
    };

    return NavigationLinkComponent;
  }, []);

  const dashboardLinkComponentResolver: DashboardLinkComponentResolver = useCallback(
    (contentPathItem: Data.ContentPathItem) => {
      const DashboardLinkComponentResolver: DashboardLinkComponent = (props) => {
        const { children } = props;
        return <Link to={`./${contentPathItemSerialize(contentPathItem)}`}>{children}</Link>;
      };

      return DashboardLinkComponentResolver;
    },
    [],
  );

  return (
    <Routes>
      <Route
        path=""
        element={
          <PageManaged
            contentPath={contentPath}
            dashboardLinkComponentResolver={dashboardLinkComponentResolver}
            navigationLinkComponentResolver={navigationLinkComponentResolver}
          />
        }
      />
      <Route path="/:contentPathItemSerialized/*" element={<ContentChild contentPath={contentPath} />} />
      <Route path="*" element={<Error404 />} />
    </Routes>
  );
};
const ContentChild: React.FC<{ contentPath: Data.ContentPath }> = (props) => {
  const { contentPath } = props;
  const params = useParams();

  const contentPathItem = contentPathItemDeserialize(params.contentPathItemSerialized!);
  const contentPathChild = contentPath.concat([contentPathItem]);

  return <Content contentPath={contentPathChild} />;
};

function contentPathItemSerialize(contentPathItem: Data.ContentPathItem): string {
  if (contentPathItem instanceof Data.ContentPathItemDashboard) {
    return `${contentPathItem.dashboardIndex}`;
  } else if (contentPathItem instanceof Data.ContentPathItemSectionDashboard) {
    return `${contentPathItem.sectionIndex}:${contentPathItem.dashboardIndex}`;
  } else {
    throw new Error("unknown contentPathItem type");
  }
}
function contentPathItemDeserialize(contentPathItemSerialized: string): Data.ContentPathItem {
  const contentPathItemSerializedItems = contentPathItemSerialized.split(":");
  if (contentPathItemSerializedItems.length === 1) {
    const dashboardIndex = parseInt(contentPathItemSerializedItems[0]);
    return new Data.ContentPathItemDashboard(dashboardIndex);
  } else if (contentPathItemSerializedItems.length === 2) {
    const sectionIndex = parseInt(contentPathItemSerializedItems[0]);
    const dashboardIndex = parseInt(contentPathItemSerializedItems[1]);
    return new Data.ContentPathItemSectionDashboard(sectionIndex, dashboardIndex);
  } else {
    throw new Error("unknown contentPathItemSerialized format");
  }
}
