import { Chip, ChipType, ChipsGroup } from "components/common/Chips";
import styled from "styled-components";

export type Data = boolean;

const Component: React.FC<{
  data: Data | undefined;
}> = (props) => {
  const { data } = props;

  return (
    <Wrapper>
      <ChipsGroup>
        <Chip type={ChipType.INFO} enabled={data === false}>
          Off
        </Chip>
        <Chip type={ChipType.INFO} enabled={data === true}>
          On
        </Chip>
      </ChipsGroup>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;
