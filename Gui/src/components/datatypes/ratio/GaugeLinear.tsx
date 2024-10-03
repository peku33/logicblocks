import GaugeLinearParent from "@/components/common/GaugeLinear";

const GaugeLinear: React.FC<{
  value: number;
  children?: React.ReactNode;
}> = (props) => {
  return <GaugeLinearParent valueMin={0.0} valueMax={1.0} valueSerializer={valueSerializer} {...props} />;
};
export default GaugeLinear;

export function valueSerializer(value: number): string {
  return `${(value * 100).toFixed(0)}%`;
}
