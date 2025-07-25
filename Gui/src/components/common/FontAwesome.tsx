import { type IconName, type IconPrefix } from "@fortawesome/fontawesome-svg-core";
import { FontAwesomeIcon as Inner } from "@fortawesome/react-fontawesome";

export interface Icon {
  prefix: IconPrefix;
  name: IconName;
}

const FontAwesomeIcon: React.FC<{
  icon: Icon;
}> = (props) => {
  const { icon } = props;

  return <Inner icon={[icon.prefix, icon.name]} />;
};
export default FontAwesomeIcon;
