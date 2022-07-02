import styled from "styled-components";
import { clamp } from "util/Number";
import Colors from "./Colors";

const GaugeLinear: React.FC<{
  value: number;
  valueMin: number;
  valueMax: number;
  valueSerializer: (value: number) => string;
  children?: React.ReactNode;
}> = (props) => {
  const { value, valueSerializer, children, valueMin, valueMax } = props;

  const valueRatio = clamp((value - valueMin) / (valueMax - valueMin), 0.0, 1.0);

  return (
    <Wrapper>
      {children !== null ? <Description>{children}</Description> : null}
      <GaugeContainer>
        <GaugeLabel>{valueSerializer(value)}</GaugeLabel>
        <GaugeBar valueRatio={valueRatio} />
      </GaugeContainer>
    </Wrapper>
  );
};
export default GaugeLinear;

const Wrapper = styled.div`
  width: 100%;
`;

const Description = styled.div`
  font-size: small;
  font-weight: bold;
`;

const GaugeContainer = styled.div`
  position: relative;

  display: flex;
  align-items: center;
  justify-content: center;

  border: solid 1px ${Colors.GREY_DARK};
  background-color: ${Colors.WHITE};
`;
const GaugeLabel = styled.div`
  font-size: small;
  margin: 0.25rem;

  z-index: 1;
`;
const GaugeBar = styled.div<{
  valueRatio: number;
}>`
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;

  width: ${(props) => (props.valueRatio * 100).toFixed(0)}%;
  transition: width 0.5s;

  background-color: ${Colors.GREY_LIGHT};
  z-index: 0;
`;
