import { ButtonActionAsync } from "@/components/common/Button";
import { Chip, ChipType } from "@/components/common/Chips";
import { Line } from "@/components/common/Line";
import styled from "styled-components";

export type Data = number | null;

const ButtonValues = [null, 0.0, 0.25, 0.5, 0.75, 1.0];
const ButtonOffsets = [0.01, 0.05, 0.1];

const Summary: React.FC<{
  data: Data | undefined; // 0.0 - 1.0
  onValueChanged: (newValue: number | null) => Promise<void>;
}> = (props) => {
  const { data, onValueChanged } = props;

  return (
    <Wrapper>
      <SetpointsLayout>
        {ButtonValues.map((buttonValue) => (
          <ButtonActionAsync
            key={buttonValue}
            onClick={async () => {
              await onValueChanged(buttonValue);
            }}
            active={buttonValue === data}
          >
            {stringifyValue(buttonValue)}
          </ButtonActionAsync>
        ))}
      </SetpointsLayout>
      <Line />
      <OffsetsLayout>
        <OffsetsLayoutItem>
          {ButtonOffsets.slice()
            .reverse()
            .map((offset) => (
              <ButtonActionAsync
                key={offset}
                active={false}
                onClick={async () => {
                  if (data == null) {
                    return;
                  }

                  await onValueChanged(fixValue(data - offset));
                }}
              >
                -{stringifyValue(offset)}
              </ButtonActionAsync>
            ))}
        </OffsetsLayoutItem>
        <OffsetsLayoutItem>
          <Chip type={ChipType.INFO} enabled={data != null && data > 0}>
            {data !== undefined ? stringifyValue(data) : "?"}
          </Chip>
        </OffsetsLayoutItem>
        <OffsetsLayoutItem>
          {ButtonOffsets.map((offset) => (
            <ButtonActionAsync
              key={offset}
              active={false}
              onClick={async () => {
                if (data == null) {
                  return;
                }

                await onValueChanged(fixValue(data + offset));
              }}
            >
              +{stringifyValue(offset)}
            </ButtonActionAsync>
          ))}
        </OffsetsLayoutItem>
      </OffsetsLayout>
    </Wrapper>
  );
};
export default Summary;

const Wrapper = styled.div``;

const SetpointsLayout = styled.div`
  display: grid;

  grid-template-columns: repeat(2, 1fr);
  grid-auto-rows: 1fr;

  grid-gap: 0.25rem;
`;

const OffsetsLayout = styled.div`
  display: grid;

  grid-template-columns: repeat(3, 1fr);

  grid-gap: 0.25rem;
`;
const OffsetsLayoutItem = styled.div`
  display: grid;

  grid-auto-rows: 1fr;

  grid-gap: 0.25rem;

  align-items: center;
`;

function fixValue(value: number): number {
  if (value > 1.0) {
    value = 1.0;
  }
  if (value < 0.0) {
    value = 0.0;
  }
  return Math.round(value * 100) / 100;
}
function stringifyValue(value: number | null): string {
  if (value === null) {
    return "Unset";
  }
  return `${(value * 100).toFixed(0)}%`;
}
