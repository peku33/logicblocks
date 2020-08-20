import React from "react";
import { Redirect, Route, Switch } from "react-router";
import { Menu } from "semantic-ui-react";
import SemanticUiReactMenuItemRouter from "components/SemanticUiReactMenuItemRouter";
import Error404 from "./Error404";
import DevicesSummary from "./DevicesSummary";
import styled from "styled-components";

const Body: React.FC = () => {
  return (
    <Layout>
      <TopBar>
        <Menu>
          <SemanticUiReactMenuItemRouter path="/device_summary" text="Devices" />
        </Menu>
      </TopBar>
      <Content>
        <Switch>
          <Route path="/device_summary">
            <DevicesSummary />
          </Route>
          <Route path="/" exact>
            <Redirect to="/device_summary" />
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
