import Dashboard from "components/dashboards/Dashboard";
import DashboardsList from "components/dashboards/DashboardsList";
import { Route, Routes, useParams } from "react-router-dom";
import Error404 from "./Error404";

const Dashboards: React.VFC = () => {
  return (
    <Routes>
      <Route path="" element={<DashboardsListRoute />} />
      <Route path=":id" element={<DashboardRoute />} />
      <Route path="*" element={<Error404 />} />
    </Routes>
  );
};
export default Dashboards;

const DashboardsListRoute: React.VFC = () => {
  return <DashboardsList />;
};
const DashboardRoute: React.VFC = () => {
  const params = useParams();

  const id = parseInt(params.id as string);
  if (Number.isNaN(id)) {
    return <Error404 />;
  }

  return <Dashboard id={id} />;
};
