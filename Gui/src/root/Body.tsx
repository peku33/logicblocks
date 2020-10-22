import Colors from "components/common/Colors";
import MediaQueries from "components/common/MediaQueries";
import React from "react";
import { matchPath, Redirect, Route, Switch, useLocation } from "react-router";
import { Link } from "react-router-dom";
import styled from "styled-components";
import DevicesSummary from "./DevicesSummary";
import Error404 from "./Error404";

const Body: React.FC = () => {
  return (
    <Layout>
      <TopBar>
        <Menu>
          <MenuItem path="/devices_summary" text="Devices" />
        </Menu>
      </TopBar>
      <Content>
        <Switch>
          <Route path="/devices_summary">
            <DevicesSummary />
          </Route>
          <Route path="/" exact>
            <Redirect to="/devices_summary" />
          </Route>
          <Route path="*">
            <Error404 />
          </Route>
        </Switch>
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
  path: string;
  exact?: boolean;
  strict?: boolean;
  sensitive?: boolean;

  text: string;
}> = (props) => {
  const location = useLocation();
  const match = !!matchPath(location.pathname, props);
  return (
    <MenuLink to={props.path} active={match}>
      {props.text}
    </MenuLink>
  );
};
const MenuLink = styled(Link)<{
  active: boolean;
}>`
  display: inline-block;
  padding: 1rem;

  color: inherit;
  text-decoration: none;
  background-color: ${Colors.BLUE};

  font-weight: bold;
`;
const Content = styled.div`
  flex: auto;
`;
