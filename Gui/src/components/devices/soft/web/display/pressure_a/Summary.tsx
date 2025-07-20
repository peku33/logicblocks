import { buildTextDisplay } from "@/components/datatypes/common/TextDisplay";
import { type Pressure, formatPressureOrUnknown } from "@/datatypes/Pressure";

export type Data = Pressure;

const TextDisplay = buildTextDisplay((value: Data | undefined) => formatPressureOrUnknown(value, 2));

const Component: React.FC<{
  data: Data | undefined;
}> = (props) => {
  const { data } = props;

  return <TextDisplay value={data} />;
};
export default Component;
