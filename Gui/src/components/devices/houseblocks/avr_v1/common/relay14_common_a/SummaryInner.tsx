import { Chip, ChipType } from "@/components/common/Chips";
import styled from "styled-components";

export const OUTPUTS_COUNT = 14;

export interface Data {
  outputs: boolean[];
}

const Component: React.FC<{
  data: Data | undefined;
}> = (props) => {
  const { data } = props;

  return (
    <RelaysGrid>
      {Array.from(Array(OUTPUTS_COUNT).keys()).map((index) => (
        <Chip key={index} type={ChipType.INFO} enabled={data?.outputs[index] || false}>
          {(index + 1).toString().padStart(2, "0")}
        </Chip>
      ))}
    </RelaysGrid>
  );
};
export default Component;

const RelaysGrid = styled.div`
  display: grid;
  grid-auto-rows: 1fr;
  grid-template-columns: repeat(7, 1fr);
  grid-gap: 0.25rem;
`;
