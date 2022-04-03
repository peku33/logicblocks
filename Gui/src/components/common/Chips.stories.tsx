import { Meta, Story } from "@storybook/react";
import styled from "styled-components";
import { Chip, ChipsGroup, ChipType } from "./Chips";

export default {
  title: "components/common/Chips",
} as Meta;

export const Basic: Story<{}> = () => (
  <>
    <Line type={ChipType.ERROR} />
    <Line type={ChipType.WARNING} />
    <Line type={ChipType.INFO} />
    <Line type={ChipType.OK} />
  </>
);
export const Group: Story<{}> = () => (
  <>
    <ChipsGroup>
      <Chip type={ChipType.ERROR}>ERROR</Chip>
      <Chip type={ChipType.WARNING} enabled>
        WARNING
      </Chip>
      <Chip type={ChipType.INFO}>INFO</Chip>
      <Chip type={ChipType.OK} enabled>
        OK
      </Chip>
    </ChipsGroup>
  </>
);

const Line: React.VFC<{ type: ChipType }> = (props) => {
  const { type } = props;

  return (
    <LineWrapper>
      <Chip type={type}>{ChipType[type]} Disabled</Chip>
      <Chip type={type} enabled>
        {ChipType[type]} Enabled
      </Chip>
    </LineWrapper>
  );
};
const LineWrapper = styled.div`
  display: flex;

  & > * {
    margin: 0.25rem;
  }
`;
