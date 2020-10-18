import React from "react";
import { Link, matchPath, useLocation } from "react-router-dom";
import { Menu } from "semantic-ui-react";

const SemanticUiReactMenuItemRouter: React.FC<{
  path: string;
  exact?: boolean;
  strict?: boolean;
  sensitive?: boolean;

  text: string;
}> = (props) => {
  const location = useLocation();
  const match = !!matchPath(location.pathname, props);
  return (
    <Menu.Item link={true} active={match}>
      <Link to={props.path}>{props.text}</Link>
    </Menu.Item>
  );
};

export default SemanticUiReactMenuItemRouter;
