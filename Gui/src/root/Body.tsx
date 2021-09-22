import Colors from "components/common/Colors";
import { matchPath, Redirect, Route, Switch, useLocation } from "react-router";
import { Link } from "react-router-dom";
import styled from "styled-components";
import DevicesSummary from "./DevicesSummary";
import Error404 from "./Error404";

const Body: React.VFC = () => {
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
const MenuItem: React.VFC<{
  path: string;
  exact?: boolean;
  strict?: boolean;
  sensitive?: boolean;

  text: string;
}> = (props) => {
  const location = useLocation();
  const match = !!matchPath(location.pathname, props);
  return (
    <MenuLink active={match}>
      <Link to={props.path}>{props.text}</Link>
    </MenuLink>
  );
};
const MenuLink = styled.div<{
  active: boolean;
}>`
  display: inline-block;
  padding: 1rem;

  background-color: ${(props) => (props.active ? Colors.BLUE : "unset")};
  color: ${(props) => (props.active ? "white" : "unset")};

  font-weight: bold;

  & > a {
    color: inherit;
    text-decoration: none;
  }
`;
const Content = styled.div`
  flex: auto;
`;
