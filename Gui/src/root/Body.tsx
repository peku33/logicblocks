import SemanticUiReactMenuItemRouter from "components/SemanticUiReactMenuItemRouter";
import React from "react";
import { Redirect, Route, Switch } from "react-router";
import { Menu } from "semantic-ui-react";
import styled from "styled-components";
import DevicesSummary from "./DevicesSummary";
import Error404 from "./Error404";

const Body: React.FC = () => {
  return (
    <Layout>
      <TopBar>
        <Menu>
          <SemanticUiReactMenuItemRouter path="/devices_summary" text="Devices" />
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
  margin-bottom: 2rem;
`;
const Content = styled.div`
  flex: auto;
`;
