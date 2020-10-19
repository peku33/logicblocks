import React from "react";
import styled from "styled-components";
import Colors from "./Colors";
import MediaQueries from "./MediaQueries";

export enum ChipState {
  UNDEFINED,

  DISABLED,
  ENABLED,

  OK,
  INFO,
  WARNING,
  ERROR,
}

export const Chip: React.FC<{
  chipState: ChipState;
}> = (props) => {
  const { chipState, children } = props;
  return <ChipOuter chipState={chipState}>{children}</ChipOuter>;
};

const ChipOuter = styled.div<{
  chipState: ChipState;
}>`
  padding: 0.25rem 0.5rem;
  border-radius: 0.25rem;

  text-align: center;

  color: white;
  background-color: ${(props): string => chipStateToBackgroundColor(props.chipState)};

  @media ${MediaQueries.COMPUTER_AT_LEAST} {
    padding: 0.5rem 1rem;
  }
`;

function chipStateToBackgroundColor(chipState: ChipState): string {
  switch (chipState) {
    case ChipState.UNDEFINED:
      return Colors.GREY;
    case ChipState.DISABLED:
      return Colors.BLUE;
    case ChipState.ENABLED:
      return Colors.GREEN;
    case ChipState.OK:
      return Colors.GREEN;
    case ChipState.INFO:
      return Colors.BLUE;
    case ChipState.WARNING:
      return Colors.YELLOW;
    case ChipState.ERROR:
      return Colors.RED;
  }
}
