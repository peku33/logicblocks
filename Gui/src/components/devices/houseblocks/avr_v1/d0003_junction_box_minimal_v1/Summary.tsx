import { kelvinToCelsius, kelvinToFahrenheit } from "lib/Temperature";
import styled from "styled-components";

interface DeviceSummary {
  temperature: number;
}

const Summary: React.VFC<{
  summary: DeviceSummary | undefined;
}> = (props) => {
  const { summary } = props;
  return (
    <Wrapper>
      <TemperatureWrapper>
        <TemperatureLabel>Temperature:</TemperatureLabel>
        <TemperatureValuePrimary>
          {summary !== undefined ? kelvinToCelsius(summary.temperature).toFixed(2) : "?"}&deg;C
        </TemperatureValuePrimary>
        <TemperatureValueSecondary>
          {summary !== undefined ? kelvinToFahrenheit(summary.temperature).toFixed(2) : "?"}&deg;F
        </TemperatureValueSecondary>
      </TemperatureWrapper>
    </Wrapper>
  );
};
export default Summary;

const Wrapper = styled.div`
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
