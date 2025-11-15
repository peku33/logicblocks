import { buildTextDisplay } from "@/components/datatypes/common/TextDisplay";
import { type FlowVolumetric, formatFlowVolumetricLitersPerMinuteOrUnknown } from "@/datatypes/FlowVolumetric";

export type Data = FlowVolumetric;

const TextDisplay = buildTextDisplay((value: Data | undefined) =>
  formatFlowVolumetricLitersPerMinuteOrUnknown(value, 3),
);

const Component: React.FC<{
  data: Data | undefined;
}> = (props) => {
  const { data } = props;

  return <TextDisplay value={data} />;
};
export default Component;
