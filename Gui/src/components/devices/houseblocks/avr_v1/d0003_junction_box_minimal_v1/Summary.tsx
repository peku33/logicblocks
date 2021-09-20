import { kelvinToCelsius, kelvinToFahrenheit } from "lib/Temperature";
import styled from "styled-components";

interface DeviceState {
  temperature: number;
}

const Summary: React.VFC<{
  state: DeviceState | undefined;
}> = (props) => {
  const { state } = props;
  return (
    <Wrapper>
      <TemperatureWrapper>
        <TemperatureLabel>Temperature:</TemperatureLabel>
        <TemperatureValuePrimary>
          {state !== undefined ? kelvinToCelsius(state.temperature).toFixed(2) : "?"}&deg;C
        </TemperatureValuePrimary>
        <TemperatureValueSecondary>
          {state !== undefined ? kelvinToFahrenheit(state.temperature).toFixed(2) : "?"}&deg;F
        </TemperatureValueSecondary>
      </TemperatureWrapper>
    </Wrapper>
  );
};
export default Summary;

const Wrapper = styled.h1`
  display: flex;
`;
const TemperatureWrapper = styled.div``;
const TemperatureLabel = styled.div`
  font-size: 0.75rem;
  text-align: right;
`;
const TemperatureValuePrimary = styled.div`
  font-size: 1.25rem;
  font-weight: bold;
  text-align: right;
`;
const TemperatureValueSecondary = styled.div`
  font-size: 0.75rem;
  font-weight: bold;
  text-align: right;
`;
