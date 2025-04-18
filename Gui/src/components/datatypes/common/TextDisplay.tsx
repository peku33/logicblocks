import React from "react";
import styled from "styled-components";

export function buildTextDisplay<T>(valueFormatter: (value: T) => string): React.FC<{
  value: T;
}> {
  const TextDisplay: React.FC<{
    value: T;
  }> = (props) => {
    const { value } = props;
    return <Value>{valueFormatter(value)}</Value>;
  };

  return TextDisplay;
}
const Value = styled.div`
  font-size: 1.5rem;

  text-align: center;
`;
