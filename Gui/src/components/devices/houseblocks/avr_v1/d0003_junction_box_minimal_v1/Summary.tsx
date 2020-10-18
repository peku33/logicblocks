import { kelvinToCelsius } from "lib/Temperature";
import React from "react";
import { Header } from "semantic-ui-react";
import makeAvrV1Summary from "../Summary";

interface DeviceState {
  temperature: number;
}

const Summary: React.FC<{
  state?: DeviceState;
}> = (props) => {
  const { state } = props;
  return <Header as="h1">{state !== undefined ? kelvinToCelsius(state.temperature) : "?"}&deg;C</Header>;
};

export default makeAvrV1Summary(Summary);
