import FontAwesomeIcon, { Icon } from "components/common/FontAwesome";
import MediaQueries from "components/common/MediaQueries";
import { getJson } from "lib/Api";
import { useState } from "react";
import { Link } from "react-router-dom";
import styled from "styled-components";
import useAsyncEffect from "use-async-effect";

const DashboardsList: React.VFC = () => {
  const dashboardsSummary = useDashboardsSummary();
  if (dashboardsSummary === undefined) {
    return null;
  }

  return (
    <List>
      {dashboardsSummary.map((dashboardSummary) => (
        <Link key={dashboardSummary.id} to={`${dashboardSummary.id}`}>
          <ListItem>
            <ListItemIcon>
              <FontAwesomeIcon icon={dashboardSummary.icon} />
            </ListItemIcon>
            <ListItemTitle>{dashboardSummary.name}</ListItemTitle>
          </ListItem>
        </Link>
      ))}
    </List>
  );
};
export default DashboardsList;

interface DashboardSummary {
  id: number;
  name: string;
  icon: Icon;
}
type DashboardsSummary = DashboardSummary[];

function useDashboardsSummary(): DashboardsSummary | undefined {
  const [dashboardsSummary, setDashboardsSummary] = useState<DashboardsSummary>();

  useAsyncEffect(
    async (isMounted) => {
      const dashboardsSummary = await getJson<DashboardsSummary>("/gui/dashboards/summary");
      if (!isMounted()) return;
      setDashboardsSummary(dashboardsSummary);
    },
    () => {
      setDashboardsSummary(undefined);
    },
    [],
  );

  return dashboardsSummary;
}

const List = styled.div`
  margin: 0.25rem;

  display: grid;
  grid-gap: 0.25rem;

  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  grid-auto-rows: 1fr;

  align-items: center;
  justify-content: center;

  & > a {
    color: inherit;
    text-decoration: none;
  }

  @media ${MediaQueries.COMPUTER_AT_LEAST} {
    margin: 0.5rem;
    grid-gap: 0.5rem;
  }
`;
const ListItem = styled.div`
  text-align: center;
  border: solid 1px black;
`;
const ListItemIcon = styled.div`
  margin: auto;

  font-size: 3rem;
  margin-bottom: 1rem;
`;
const ListItemTitle = styled.h3`
  font-size: 2rem;
`;
