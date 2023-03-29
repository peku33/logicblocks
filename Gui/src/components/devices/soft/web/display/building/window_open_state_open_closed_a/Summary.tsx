import { WindowOpenStateOpenClosedComponent } from "components/datatypes/building/Window";
import { WindowOpenStateOpenClosed } from "datatypes/building/Window";

export type Data = WindowOpenStateOpenClosed;

const Component: React.FC<{
  data: Data | undefined;
}> = (props) => {
  const { data } = props;

  return <WindowOpenStateOpenClosedComponent value={data} />;
};
export default Component;
