import DeviceSummaryManagedWrapperManagedList from "components/devices/DeviceSummaryManagedWrapperManagedList";
import { DashboardLinkComponentResolver, NavigationLinkComponentResolver, Page } from "./Dashboards";
import * as Data from "./Data";

export const PageManaged: React.FC<{
  contentPath: Data.ContentPath;
  navigationLinkComponentResolver: NavigationLinkComponentResolver;
  dashboardLinkComponentResolver: DashboardLinkComponentResolver;
}> = (props) => {
  const { contentPath, navigationLinkComponentResolver, dashboardLinkComponentResolver } = props;

  const navigation = Data.useNavigation(contentPath);
  const dashboardContent = Data.useDashboardContent(contentPath);

  return (
    <Page
      navigation={navigation}
      navigationLinkComponentResolver={navigationLinkComponentResolver}
      dashboardContent={dashboardContent}
      dashboardLinkComponentResolver={dashboardLinkComponentResolver}
      deviceListComponent={DeviceSummaryManagedWrapperManagedList}
    />
  );
};
