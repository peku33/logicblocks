import { Button } from "components/common/Button";
import styled from "styled-components";

const ButtonValues = [null, 0.0, 0.25, 0.5, 0.75, 1.0];
const ButtonOffsets = [0.01, 0.05, 0.1];

const Summary: React.VFC<{
  value: number | null | undefined; // 0.0 - 1.0
  onValueChanged: ((newValue: number | null) => void) | undefined;
}> = (props) => {
  const { value, onValueChanged } = props;

  return (
    <Wrapper>
      <LayoutLine>
        {ButtonValues.map((buttonValue) => (
          <Button
            key={buttonValue}
            onClick={onValueChanged !== undefined ? () => onValueChanged(buttonValue) : () => ({})}
            active={buttonValue === value}
          >
            {stringifyValue(buttonValue)}
          </Button>
        ))}
      </LayoutLine>
      <LayoutLine>
        {ButtonOffsets.slice()
          .reverse()
          .map((offset) => (
            <Button
              key={offset}
              onClick={
                onValueChanged !== undefined && value !== undefined && value !== null
                  ? () => onValueChanged(fixValue(value - offset))
                  : () => ({})
              }
            >
              -{stringifyValue(offset)}
            </Button>
          ))}
        <CurrentValue>
          <CurrentValueLabel>CurrentValue</CurrentValueLabel>
          <CurrentValueValue>{value !== undefined ? stringifyValue(value) : "?"}</CurrentValueValue>
        </CurrentValue>
        {ButtonOffsets.map((offset) => (
          <Button
            key={offset}
            onClick={
              onValueChanged !== undefined && value !== undefined && value !== null
                ? () => onValueChanged(fixValue(value + offset))
                : () => ({})
            }
          >
            +{stringifyValue(offset)}
          </Button>
        ))}
      </LayoutLine>
    </Wrapper>
  );
};
export default Summary;

const Wrapper = styled.div`
  display: flex;
  flex-direction: column;
`;
const LayoutLine = styled.div`
  display: flex;
  justify-content: center;
  margin: 0.25rem;
`;

const CurrentValue = styled.div`
  margin: 0 1rem;
  text-align: center;
`;
const CurrentValueLabel = styled.div``;
const CurrentValueValue = styled.div``;

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
