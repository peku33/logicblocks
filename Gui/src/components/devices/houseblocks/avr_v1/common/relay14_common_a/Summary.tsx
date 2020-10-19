import { kelvinToCelsius } from "lib/Temperature";
import React from "react";
import { Header } from "semantic-ui-react";
import makeAvrV1Summary from "../../Summary";

interface DeviceState {}

const Summary: React.FC<{
  state?: DeviceState;
}> = (props) => {
  const { state } = props;
  return <p>Relays?</p>;
};

export default makeAvrV1Summary(Summary);
