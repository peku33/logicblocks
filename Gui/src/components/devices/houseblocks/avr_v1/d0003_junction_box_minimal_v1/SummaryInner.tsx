import { kelvinToCelsius, kelvinToFahrenheit } from "datatypes/Temperature";
import styled from "styled-components";

export interface Data {
  temperature: number;
}

const Component: React.FC<{
  data: Data | undefined;
}> = (props) => {
  const { data } = props;

  return (
    <Wrapper>
      <TemperatureWrapper>
        <TemperatureLabel>Temperature:</TemperatureLabel>
        <TemperatureValuePrimary>
          {data !== undefined ? kelvinToCelsius(data.temperature).toFixed(2) : "?"}&deg;C
        </TemperatureValuePrimary>
        <TemperatureValueSecondary>
          {data !== undefined ? kelvinToFahrenheit(data.temperature).toFixed(2) : "?"}&deg;F
        </TemperatureValueSecondary>
      </TemperatureWrapper>
    </Wrapper>
  );
};
export default Component;

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
