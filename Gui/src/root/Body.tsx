import Colors from "components/common/Colors";
import { Link, Navigate, Route, Routes, useMatch } from "react-router-dom";
import styled from "styled-components";
import DevicesSummary from "./DevicesSummary";
import Error404 from "./Error404";

const Body: React.VFC = () => {
  return (
    <Layout>
      <TopBar>
        <Menu>
          <MenuItem pattern="/devices_summary/*" target="/devices_summary" text="Devices" />
        </Menu>
      </TopBar>
      <Content>
        <Routes>
          <Route path="devices_summary/*" element={<DevicesSummary />} />
          <Route path="" element={<Navigate to="devices_summary" />} />
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
const MenuItem: React.VFC<{
  pattern: string;
  target: string;
  text: string;
}> = (props) => {
  const match = useMatch(props.pattern) !== null;

  return (
    <MenuLink active={match}>
      <Link to={props.target}>{props.text}</Link>
    </MenuLink>
  );
};
const MenuLink = styled.div<{
  active: boolean;
}>`
  display: inline-block;
  padding: 1rem;

  background-color: ${(props) => (props.active ? Colors.BLUE : "unset")};
  color: ${(props) => (props.active ? Colors.WHITE : "unset")};

  font-weight: bold;

  & > a {
    color: inherit;
    text-decoration: none;
  }
`;
const Content = styled.div`
  flex: auto;
`;
