import { ButtonActionAsync } from "@/components/common/Button";
import Colors from "@/components/common/Colors";
import { formatRealOrUnknown } from "@/datatypes/Real";
import { useCallback, useMemo, useState } from "react";
import styled from "styled-components";

export type Data = number | null;

const Summary: React.FC<{
  data: Data | undefined; // 0.0 - 1.0
  onValueChanged: (newValue: number | null) => Promise<void>;
}> = (props) => {
  const { data, onValueChanged } = props;

  // value from input field, as is
  const [inputValue, setInputValue] = useState<string | undefined>();

  // parsed value from input field, or null if empty, or undefined if invalid
  const value = useMemo(() => {
    if (inputValue == null || inputValue === "") {
      return null;
    }

    const value = parseFloat(inputValue);
    if (!Number.isFinite(value)) {
      return undefined;
    }

    return value;
  }, [inputValue]);

  const onClick = useCallback(async () => {
    if (value === undefined) return;

    await onValueChanged(value);
  }, [value, onValueChanged]);

  return (
    <Wrapper>
      <ValueWrapper>
        <ValueWrapperLabel>Current Value:</ValueWrapperLabel>
        <ValueWrapperValue>{formatRealOrUnknown(data, undefined)}</ValueWrapperValue>
      </ValueWrapper>
      <SetterWrapper>
        <Input
          value={inputValue}
          onChange={(event) => {
            setInputValue(event.target.value);
          }}
        />
        <ButtonActionAsync active={value !== undefined} onClick={onClick}>
          Set
        </ButtonActionAsync>
      </SetterWrapper>
    </Wrapper>
  );
};
export default Summary;

const Wrapper = styled.div``;

const ValueWrapper = styled.div`
  display: grid;
  grid-gap: 0.25rem;
  grid-auto-flow: column;

  align-items: center;
  justify-content: center;

  font-size: 0.75rem;

  margin-bottom: 0.5rem;
`;
const ValueWrapperLabel = styled.div``;
const ValueWrapperValue = styled.div`
  font-weight: bold;
`;

const SetterWrapper = styled.div`
  display: grid;
  grid-gap: 0.5rem;
  grid-auto-flow: column;

  align-items: center;
  justify-content: center;

  margin-top: 0.5rem;
`;

const Input = styled.input.attrs((_props) => ({
  type: "number",
  step: "any",
}))`
  border: solid 1px ${Colors.GREY};
  border-radius: 0.25rem;

  outline: none;

  padding: 0.5rem;
`;
