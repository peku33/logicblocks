import React, { Fragment, useEffect, useState } from "react";
import { Button, Container, Divider, Header, Icon, Image, Label, Loader } from "semantic-ui-react";
import { DeviceEventsManager, DeviceStateManager } from "../../../services/DevicePool";
import DeviceContext from "../DeviceContext";

interface Event {
  type: string;
  region?: string;
  rule_id?: number;
  direction?: string;
}
function eventToString(event: Event): string {
  const details = [event.region, event.rule_id, event.direction]
    .filter((item) => item !== undefined)
    .join(", ");
  return event.type + (details ? ` (${details})` : "");
}

interface State {
  device_name: string;
  state: string;
  snapshot_available: boolean;
  events: Event[];
  rtsp_streams: {
    [name: string]: string,
  };
}

const STREAM_NAMES = {
  main: "High",
  sub1: "Medium",
  sub2: "Low",
};

const STATE_COLORS = {
  Initializing: "yellow",
  Configuring: "yellow",
  Running: "green",
  Error: "red",
};
const STATE_COLORS_UNKNOWN = "orange";

const STATE_ICONS = {
  Initializing: "setting",
  Configuring: "setting",
  Running: "play",
  Error: "x",
};
const STATE_ICON_UNKNOWN = "question";

const Ipc: React.FC<{
  deviceContext: DeviceContext,
}> = (props) => {
  const [deviceState, setDeviceState] = useState<State>();
  useEffect(() => (new DeviceStateManager<State>(props.deviceContext.deviceId)).reactHook(setDeviceState), [props.deviceContext.deviceId]);

  const [snapshotLastUpdate, setSnapshotLastUpdate] = useState(new Date());
  useEffect(() => (new DeviceEventsManager(props.deviceContext.deviceId).reactHook((event) => {
    switch (event) {
      case "snapshot": setSnapshotLastUpdate(new Date()); break;
      default: console.warn("Unrecognized event", event);
    }
  })), [props.deviceContext.deviceId]);

  if (!deviceState) { return <Loader active />; }
  return (
    <Fragment>
      <Header as="h1">{deviceState.device_name}</Header>
      <Divider />
      {deviceState.snapshot_available ? (
        <Image
          src={props.deviceContext.urlBuild(`/snapshot/small?timestamp=${snapshotLastUpdate.getTime()}`)}
          centered fluid
          href={props.deviceContext.urlBuild("/snapshot/")}
        />
      ) : (
          <Loader active inline="centered" />
        )}
      <Divider />
      <Label color={(STATE_COLORS as any)[deviceState.state] || STATE_COLORS_UNKNOWN}>
        <Icon name={(STATE_ICONS as any)[deviceState.state] || STATE_ICON_UNKNOWN} />
        {deviceState.state}
      </Label>
      <Divider />
      {deviceState.events.length ? (
        deviceState.events.map((event) => {
          const eventString = eventToString(event);
          return (
            <Label key={eventString} color="orange">
              <Icon name="exclamation" />
              {eventString}
            </Label>
          );
        })
      ) : (
          <Label color="green">
            <Icon name="check" />
            No events
      </Label>
        )}
      <Divider />
      <Container textAlign="center">
        {Object.entries(deviceState.rtsp_streams).map(([key, url]) => (
          <Button key={key} as="a" href={url}>
            {(STREAM_NAMES as any)[key] || key}
          </Button>
        ))}
      </Container>
    </Fragment>
  );
};

export default Ipc;