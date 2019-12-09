import React from "react";
import { Redirect, Route, Switch } from "react-router";
import { Menu } from "semantic-ui-react";
import DevicePool from "../components/devices/DevicePool";
import SemanticUiReactMenuItemRouter from "../components/SemanticUiReactMenuItemRouter";
import Error404 from "./Error404";

const Body: React.FC = () => {
  return (
    <div>
      <Menu>
        <SemanticUiReactMenuItemRouter path="/device_pool" text="Devices" />
      </Menu>
      <Switch>
        <Route path="/device_pool">
          <DevicePool />
        </Route>
        <Route path="/" exact>
          <Redirect to="/device_pool" />
        </Route>
        <Route path="*">
          <Error404 />
        </Route>
      </Switch>
    </div>
  );
};

export default Body;
