import { Chip, ChipType } from "components/common/Chips";
import MediaQueries from "components/common/MediaQueries";
import React from "react";
import styled from "styled-components";
import makeAvrV1Summary from "../../Summary";

const OUTPUT_COUNT = 14;

interface DeviceState {
  values: boolean[];
}

const Summary: React.FC<{
  state?: DeviceState;
}> = (props) => {
  const { state } = props;
  return (
    <RelaysGrid>
      {Array.from(Array(OUTPUT_COUNT).keys()).map((index) => (
        <Chip key={index} type={ChipType.INFO} enabled={state?.values[index] || false}>
          {(index + 1).toString().padStart(2, "0")}
        </Chip>
      ))}
    </RelaysGrid>
  );
};

export default makeAvrV1Summary(Summary);

const RelaysGrid = styled.div`
  display: grid;
  grid-auto-rows: 1fr;
  grid-template-columns: repeat(7, 1fr);
  grid-gap: 0.25rem;

  @media ${MediaQueries.COMPUTER_ONLY} {
    grid-template-columns: repeat(14, 1fr);
  }
`;
