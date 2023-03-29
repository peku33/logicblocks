import { WindowOpenStateOpenTiltedClosedComponent } from "components/datatypes/building/Window";
import { WindowOpenStateOpenTiltedClosed } from "datatypes/building/Window";

export type Data = WindowOpenStateOpenTiltedClosed;

const Component: React.FC<{
  data: Data | undefined;
}> = (props) => {
  const { data } = props;

  return <WindowOpenStateOpenTiltedClosedComponent value={data} />;
};
export default Component;
