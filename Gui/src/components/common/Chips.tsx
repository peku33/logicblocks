import styled from "styled-components";
import Colors from "./Colors";

export enum ChipType {
  OK,
  INFO,
  WARNING,
  ERROR,
}

function enabledColor(type: ChipType): string {
  switch (type) {
    case ChipType.OK:
      return Colors.GREEN;
    case ChipType.INFO:
      return Colors.BLUE;
    case ChipType.WARNING:
      return Colors.ORANGE;
    case ChipType.ERROR:
      return Colors.RED;
  }
}

const ChipInner = styled.div<{
  type: ChipType;
  enabled?: boolean;
}>`
  padding: 0.25rem 0.5rem;
  text-align: center;

  border-radius: 0.25rem;
  border: solid 1px ${(props) => enabledColor(props.type)};

  color: ${(props) => (props.enabled ? Colors.WHITE : enabledColor(props.type))};
  background-color: ${(props) => (props.enabled ? enabledColor(props.type) : Colors.WHITE)};
`;
export const Chip: React.FC<{
  type: ChipType;
  enabled?: boolean;
}> = (props) => {
  const { type, enabled, children } = props;

  return (
    <ChipInner type={type} enabled={enabled}>
      {children}
    </ChipInner>
  );
};

export const ChipsGroup = styled.div`
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  align-items: center;

  margin: -0.125rem;
  & > ${ChipInner} {
    flex: auto;
    margin: 0.125rem;
  }
`;
