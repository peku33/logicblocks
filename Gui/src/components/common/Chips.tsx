import styled from "styled-components";
import Colors from "./Colors";
import MediaQueries from "./MediaQueries";

export enum ChipType {
  OK,
  INFO,
  WARNING,
  ERROR,
}

export const Chip = styled.div<{
  type: ChipType;
  enabled: boolean;
}>`
  padding: 0.25rem 0.5rem;
  text-align: center;

  border-radius: 0.25rem;
  border: solid 1px ${(props) => enabledColor(props.type)};

  color: ${(props) => (props.enabled ? Colors.WHITE : enabledColor(props.type))};
  background-color: ${(props) => (props.enabled ? enabledColor(props.type) : Colors.WHITE)};

  @media ${MediaQueries.COMPUTER_AT_LEAST} {
    padding: 0.5rem 1rem;
  }
`;
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

export const ChipsGroup = styled.div`
  display: flex;
  align-items: center;

  margin: 0 -0.5rem;
  & > ${Chip} {
    margin: 0 0.5rem;

    @media ${MediaQueries.COMPUTER_AT_LEAST} {
      margin: 0 0.25rem;
    }
  }

  @media ${MediaQueries.COMPUTER_AT_LEAST} {
    margin: 0 -0.25rem;
  }
`;
