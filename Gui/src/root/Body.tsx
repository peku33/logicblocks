import Colors from "components/common/Colors";
import { Navigate, NavLink, Route, Routes, To } from "react-router-dom";
import styled from "styled-components";
import Dashboards from "./Dashboards";
import DevicesSummary from "./DevicesSummary";
import Error404 from "./Error404";

const Body: React.FC<{}> = () => {
  return (
    <Layout>
      <TopBar>
        <Menu>
          <MenuItem to="/dashboards">Dashboards</MenuItem>
          <MenuItem to="/devices_summary">Devices</MenuItem>
        </Menu>
      </TopBar>
      <Content>
        <Routes>
          <Route path="dashboards/*" element={<Dashboards />} />
          <Route path="devices_summary/*" element={<DevicesSummary />} />
          <Route path="" element={<Navigate to="/dashboards" />} />
          <Route path="*" element={<Error404 />} />
        </Routes>
      </Content>
    </Layout>
  );
};
export default Body;

const Layout = styled.div`
  height: 100%;
  display: flex;
  flex-direction: column;
`;

const TopBar = styled.div`
  border-bottom: solid 1px ${Colors.GREY_LIGHTEST};
`;
const Menu = styled.div``;
const MenuItem: React.FC<{
  to: To;
  children?: React.ReactNode;
}> = (props) => {
  const { to, children } = props;

  return <NavLink to={to}>{({ isActive }) => <MenuLink active={isActive}>{children}</MenuLink>}</NavLink>;
};
const MenuLink = styled.div<{
  active: boolean;
}>`
  display: inline-block;
  padding: 1rem;

  background-color: ${(props) => (props.active ? Colors.BLUE : "unset")};
  color: ${(props) => (props.active ? Colors.WHITE : "inherit")};

  font-weight: bold;
`;

const Content = styled.div`
  flex: auto;
`;
