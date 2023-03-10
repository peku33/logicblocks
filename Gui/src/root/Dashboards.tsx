import {
  DashboardLinkComponent,
  DashboardLinkComponentResolver,
  NavigationLinkComponent,
  NavigationLinkComponentResolver,
} from "components/dashboards/Dashboards";
import { PageManaged } from "components/dashboards/DashboardsManaged";
import * as Data from "components/dashboards/Data";
import { useCallback } from "react";
import { Link, Route, Routes, useParams } from "react-router-dom";
import Error404 from "./Error404";

const Root: React.FC<{}> = () => {
  return <Content contentPath={[]} />;
};
export default Root;

const Content: React.FC<{ contentPath: Data.ContentPath }> = (props) => {
  const { contentPath } = props;

  const navigationLinkComponentResolver: NavigationLinkComponentResolver = useCallback((backDepth: number) => {
    const NavigationLinkComponent: NavigationLinkComponent = (props) => {
      const { children } = props;
      return <Link to={`.${"/../..".repeat(backDepth)}`}>{children}</Link>;
    };

    return NavigationLinkComponent;
  }, []);

  const dashboardLinkComponentResolver: DashboardLinkComponentResolver = useCallback(
    (contentPathItem: Data.ContentPathItem) => {
      const DashboardLinkComponentResolver: DashboardLinkComponent = (props) => {
        const { children } = props;
        return <Link to={`./${contentPathItem.section_index}/${contentPathItem.dashboard_index}`}>{children}</Link>;
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
      <Route path="/:sectionId/:dashboardId/*" element={<ContentChild contentPath={contentPath} />} />
      <Route path="*" element={<Error404 />} />
    </Routes>
  );
};
const ContentChild: React.FC<{ contentPath: Data.ContentPath }> = (props) => {
  const { contentPath } = props;
  const params = useParams();

  const contentPathItem: Data.ContentPathItem = {
    section_index: parseInt(params.sectionId as string),
    dashboard_index: parseInt(params.dashboardId as string),
  };
  const contentPathChild = contentPath.concat([contentPathItem]);

  return <Content contentPath={contentPathChild} />;
};
