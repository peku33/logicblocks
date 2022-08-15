import Colors from "components/common/Colors";
import FontAwesomeIcon, { Icon } from "components/common/FontAwesome";
import { DeviceId } from "components/devices/Device";
import DeviceSummaryManagedWrapperList from "components/devices/DeviceSummaryManagedWrapperList";
import { getJson } from "lib/Api";
import { useState } from "react";
import { Link } from "react-router-dom";
import styled from "styled-components";
import useAsyncEffect from "use-async-effect";

const Dashboard: React.FC<{
  id: number;
}> = (props) => {
  const { id } = props;

  const dashboardSummary = useDashboardSummary(id);
  if (dashboardSummary === undefined) {
    return null;
  }

  return (
    <Wrapper>
      <Header>
        <Link to="..">&lt;</Link>
        <FontAwesomeIcon icon={dashboardSummary.icon} />
        <Title>{dashboardSummary.name}</Title>
      </Header>
      <ListWrapper>
        <DeviceSummaryManagedWrapperList deviceIds={dashboardSummary.device_ids} />
      </ListWrapper>
    </Wrapper>
  );
};
export default Dashboard;

interface DashboardSummary {
  name: string;
  icon: Icon;
  device_ids: DeviceId[];
}

function useDashboardSummary(id: number): DashboardSummary | undefined {
  const [dashboardSummary, setDashboardSummary] = useState<DashboardSummary>();

  useAsyncEffect(
    async (isMounted) => {
      const dashboardsSummary = await getJson<DashboardSummary>(`/gui/dashboards/${id}/summary`);
      if (!isMounted()) return;
      setDashboardSummary(dashboardsSummary);
    },
    [id],
  );

  return dashboardSummary;
}

const Wrapper = styled.div``;

const Header = styled.div`
  display: flex;
  align-items: center;
  padding: 0.25rem;

  border-bottom: solid 1px ${Colors.GREY_LIGHTEST};

  & > a {
    margin-right: 1rem;

    text-decoration: None;
    color: inherit;
  }
`;
const Title = styled.h4`
  margin-left: 0.25rem;
`;

const ListWrapper = styled.div``;
