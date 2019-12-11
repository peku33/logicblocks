import React, { Fragment, useEffect, useState } from "react";
import { Button, Divider, Header, Icon, Label, Loader, Table } from "semantic-ui-react";
import { postJsonEmpty } from "../../../../services/Api";
import { DeviceStateManager } from "../../../../services/DevicePool";
import DeviceContext from "../../DeviceContext";

interface StateInitializing {
  state: "Initializing";
}
interface StateRunning {
  state: "Running";
  relay_states: [boolean];
}
interface StateError {
  state: "Error";
}
interface State {
  desired_relay_states: [boolean];
  state: StateInitializing | StateRunning | StateError;
}

const STATE_COLORS = {
  Initializing: "yellow",
  Running: "green",
  Error: "red",
};
const STATE_COLORS_UNKNOWN = "orange";

const STATE_ICONS = {
  Initializing: "setting",
  Running: "play",
  Error: "x",
};
const STATE_ICON_UNKNOWN = "question";

const Relay14Common: React.FC<{
  deviceContext: DeviceContext,
  deviceClass: string,
}> = (props) => {
  const [deviceState, setDeviceState] = useState<State>();
  useEffect(() => (new DeviceStateManager<State>(props.deviceContext.deviceId)).reactHook(setDeviceState), [props.deviceContext.deviceId]);

  const relayStateTransition = async (id: number, state: boolean) => {
    await postJsonEmpty(
      props.deviceContext.endpointBuild("/relay_state_transition"), {
      id,
      state,
    });
  };

  if (!deviceState) { return <Loader active />; }

  const compactedRelayStates = deviceState.desired_relay_states.map((relayState, index) => ({
    desired: relayState,
    actual: deviceState.state.state === "Running" ? deviceState.state.relay_states[index] : undefined,
  }));

  return (
    <Fragment>
      <Header as="h5">{props.deviceClass}</Header>
      <Divider />
      <Label color={(STATE_COLORS as any)[deviceState.state.state] || STATE_COLORS_UNKNOWN}>
        <Icon name={(STATE_ICONS as any)[deviceState.state.state] || STATE_ICON_UNKNOWN} />
        {deviceState.state.state}
      </Label>
      <Divider />
      <Table singleLine unstackable>
        <Table.Header>
          <Table.Row>
            <Table.HeaderCell>Channel</Table.HeaderCell>
            <Table.HeaderCell>Desired</Table.HeaderCell>
            <Table.HeaderCell>Actual</Table.HeaderCell>
          </Table.Row>
        </Table.Header>
        <Table.Body>
          {compactedRelayStates.map((relay, id) => (
            <Table.Row key={id}>
              <Table.Cell>
                {id}
              </Table.Cell>
              <Table.Cell>
                <Button
                  onClick={() => relayStateTransition(id, !relay.desired)}
                  color={relay.desired ? "green" : "grey"}
                  compact
                >
                  <Icon name="bolt" />
                  {relay.desired ? "On" : "Off"}
                </Button>
              </Table.Cell>
              <Table.Cell>
                <Label color={relay.actual ? "green" : "grey"}>
                  <Icon name={relay.actual === undefined ? "question" : "bolt"} />
                  {relay.actual === undefined ? "Unknown" : (relay.actual ? "On" : "Off")}
                </Label>
              </Table.Cell>
            </Table.Row>
          ))}
        </Table.Body>
      </Table>
    </Fragment>
  );
};

export default Relay14Common;
