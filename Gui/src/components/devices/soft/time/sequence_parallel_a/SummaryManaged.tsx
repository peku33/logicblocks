import { Button, ButtonGroup } from "components/common/Button";
import { Chip, ChipType } from "components/common/Chips";
import Colors from "components/common/Colors";
import { SummaryManagedBase } from "components/devices/SummaryManaged";
import { useCallback } from "react";
import { devicePostEmpty, devicePostJsonEmpty, useDeviceSummary } from "services/LogicDevicesRunner";
import styled from "styled-components";

interface DeviceSummaryChannelConfiguration {
  name: string;

  base_time_seconds: number;
  power_required: number;

  round_min_seconds: number;
  round_max_seconds: number;
}
interface DeviceSummaryConfiguration {
  channels: DeviceSummaryChannelConfiguration[];
  power_max: number;
}

// DeviceSummaryDeviceStateDisabledChannelState
interface DeviceSummaryDeviceStateDisabledChannelStateDisabled {
  state: "Disabled";
}
interface DeviceSummaryDeviceStateDisabledChannelStatePaused {
  state: "Paused";
}
interface DeviceSummaryDeviceStateDisabledChannelStateEnabled {
  state: "Enabled";
}
type DeviceSummaryDeviceStateDisabledChannelState =
  | DeviceSummaryDeviceStateDisabledChannelStateDisabled
  | DeviceSummaryDeviceStateDisabledChannelStatePaused
  | DeviceSummaryDeviceStateDisabledChannelStateEnabled;
function deviceSummaryDeviceStateDisabledChannelStateIsDisabled(
  deviceSummaryDeviceStateDisabledChannelState: DeviceSummaryDeviceStateDisabledChannelState,
): deviceSummaryDeviceStateDisabledChannelState is DeviceSummaryDeviceStateDisabledChannelStateDisabled {
  return deviceSummaryDeviceStateDisabledChannelState.state === "Disabled";
}
function deviceSummaryDeviceStateDisabledChannelStateIsPaused(
  deviceSummaryDeviceStateDisabledChannelState: DeviceSummaryDeviceStateDisabledChannelState,
): deviceSummaryDeviceStateDisabledChannelState is DeviceSummaryDeviceStateDisabledChannelStatePaused {
  return deviceSummaryDeviceStateDisabledChannelState.state === "Paused";
}
function deviceSummaryDeviceStateDisabledChannelStateIsEnabled(
  deviceSummaryDeviceStateDisabledChannelState: DeviceSummaryDeviceStateDisabledChannelState,
): deviceSummaryDeviceStateDisabledChannelState is DeviceSummaryDeviceStateDisabledChannelStateEnabled {
  return deviceSummaryDeviceStateDisabledChannelState.state === "Enabled";
}

// DeviceSummaryDeviceStatePausedChannelState
interface DeviceSummaryDeviceStatePausedChannelStateDisabled {
  state: "Disabled";
}
interface DeviceSummaryDeviceStatePausedChannelStatePaused {
  state: "Paused";
  queue_seconds: number;
}
interface DeviceSummaryDeviceStatePausedChannelStateEnabled {
  state: "Enabled";
  queue_seconds: number;
}
type DeviceSummaryDeviceStatePausedChannelState =
  | DeviceSummaryDeviceStatePausedChannelStateDisabled
  | DeviceSummaryDeviceStatePausedChannelStatePaused
  | DeviceSummaryDeviceStatePausedChannelStateEnabled;
function deviceSummaryDeviceStatePausedChannelStateIsDisabled(
  deviceSummaryDeviceStatePausedChannelState: DeviceSummaryDeviceStatePausedChannelState,
): deviceSummaryDeviceStatePausedChannelState is DeviceSummaryDeviceStatePausedChannelStateDisabled {
  return deviceSummaryDeviceStatePausedChannelState.state === "Disabled";
}
function deviceSummaryDeviceStatePausedChannelStateIsPaused(
  deviceSummaryDeviceStatePausedChannelState: DeviceSummaryDeviceStatePausedChannelState,
): deviceSummaryDeviceStatePausedChannelState is DeviceSummaryDeviceStatePausedChannelStatePaused {
  return deviceSummaryDeviceStatePausedChannelState.state === "Paused";
}
function deviceSummaryDeviceStatePausedChannelStateIsEnabled(
  deviceSummaryDeviceStatePausedChannelState: DeviceSummaryDeviceStatePausedChannelState,
): deviceSummaryDeviceStatePausedChannelState is DeviceSummaryDeviceStatePausedChannelStateEnabled {
  return deviceSummaryDeviceStatePausedChannelState.state === "Enabled";
}

// DeviceSummaryDeviceStateEnabledChannelState
interface DeviceSummaryDeviceStateEnabledChannelStateDisabled {
  state: "Disabled";
}
interface DeviceSummaryDeviceStateEnabledChannelStatePaused {
  state: "Paused";
  queue_seconds: number;
}
interface DeviceSummaryDeviceStateEnabledChannelStateEnabledQueued {
  state: "EnabledQueued";
  queue_seconds: number;
  queue_position: number | null;
}
interface DeviceSummaryDeviceStateEnabledChannelStateEnabledActive {
  state: "EnabledActive";
  queue_seconds: number;
  round_seconds: number;
}
type DeviceSummaryDeviceStateEnabledChannelState =
  | DeviceSummaryDeviceStateEnabledChannelStateDisabled
  | DeviceSummaryDeviceStateEnabledChannelStatePaused
  | DeviceSummaryDeviceStateEnabledChannelStateEnabledQueued
  | DeviceSummaryDeviceStateEnabledChannelStateEnabledActive;

function deviceSummaryDeviceStateEnabledChannelStateIsDisabled(
  deviceSummaryDeviceStateEnabledChannelState: DeviceSummaryDeviceStateEnabledChannelState,
): deviceSummaryDeviceStateEnabledChannelState is DeviceSummaryDeviceStateEnabledChannelStateDisabled {
  return deviceSummaryDeviceStateEnabledChannelState.state === "Disabled";
}
function deviceSummaryDeviceStateEnabledChannelStateIsPaused(
  deviceSummaryDeviceStateEnabledChannelState: DeviceSummaryDeviceStateEnabledChannelState,
): deviceSummaryDeviceStateEnabledChannelState is DeviceSummaryDeviceStateEnabledChannelStatePaused {
  return deviceSummaryDeviceStateEnabledChannelState.state === "Paused";
}
function deviceSummaryDeviceStateEnabledChannelStateIsEnabledQueued(
  deviceSummaryDeviceStateEnabledChannelState: DeviceSummaryDeviceStateEnabledChannelState,
): deviceSummaryDeviceStateEnabledChannelState is DeviceSummaryDeviceStateEnabledChannelStateEnabledQueued {
  return deviceSummaryDeviceStateEnabledChannelState.state === "EnabledQueued";
}
function deviceSummaryDeviceStateEnabledChannelStateIsEnabledActive(
  deviceSummaryDeviceStateEnabledChannelState: DeviceSummaryDeviceStateEnabledChannelState,
): deviceSummaryDeviceStateEnabledChannelState is DeviceSummaryDeviceStateEnabledChannelStateEnabledActive {
  return deviceSummaryDeviceStateEnabledChannelState.state === "EnabledActive";
}

// DeviceSummaryState
interface DeviceSummaryStateDisabled {
  state: "Disabled";
  channels: DeviceSummaryDeviceStateDisabledChannelState[];
}
interface DeviceSummaryStatePaused {
  state: "Paused";
  channels: DeviceSummaryDeviceStatePausedChannelState[];
}
interface DeviceSummaryStateEnabled {
  state: "Enabled";
  channels: DeviceSummaryDeviceStateEnabledChannelState[];
  power: number;
}
type DeviceSummaryState = DeviceSummaryStateDisabled | DeviceSummaryStatePaused | DeviceSummaryStateEnabled;
function deviceSummaryStateIsDisabled(
  deviceSummaryState: DeviceSummaryState,
): deviceSummaryState is DeviceSummaryStateDisabled {
  return deviceSummaryState.state === "Disabled";
}
function deviceSummaryStateIsPaused(
  deviceSummaryState: DeviceSummaryState,
): deviceSummaryState is DeviceSummaryStatePaused {
  return deviceSummaryState.state === "Paused";
}
function deviceSummaryStateIsEnabled(
  deviceSummaryState: DeviceSummaryState,
): deviceSummaryState is DeviceSummaryStateEnabled {
  return deviceSummaryState.state === "Enabled";
}

// DeviceSummary
interface DeviceSummary {
  configuration: DeviceSummaryConfiguration;
  state: DeviceSummaryState;
}

const SummaryManaged: SummaryManagedBase = (props) => {
  const { deviceId } = props;

  const deviceSummary = useDeviceSummary<DeviceSummary>(deviceId);

  const onDeviceDisable = useCallback((): void => {
    devicePostEmpty(deviceId, "/device/disable");
  }, [deviceId]);
  const onDevicePause = useCallback((): void => {
    devicePostEmpty(deviceId, "/device/pause");
  }, [deviceId]);
  const onDeviceEnable = useCallback((): void => {
    devicePostEmpty(deviceId, "/device/enable");
  }, [deviceId]);

  const onChannelsAllClear = useCallback((): void => {
    devicePostEmpty(deviceId, "/channels/all/clear");
  }, [deviceId]);
  const onChannelsAllAdd = useCallback(
    (multiplier: number): void => {
      devicePostJsonEmpty(deviceId, "/channels/all/add", multiplier);
    },
    [deviceId],
  );

  const onChannelDisable = useCallback(
    (channelId: number) => {
      devicePostEmpty(deviceId, `/channels/${channelId}/disable`);
    },
    [deviceId],
  );
  const onChannelPause = useCallback(
    (channelId: number) => {
      devicePostEmpty(deviceId, `/channels/${channelId}/pause`);
    },
    [deviceId],
  );
  const onChannelEnable = useCallback(
    (channelId: number) => {
      devicePostEmpty(deviceId, `/channels/${channelId}/enable`);
    },
    [deviceId],
  );
  const onChannelClear = useCallback(
    (channelId: number) => {
      devicePostEmpty(deviceId, `/channels/${channelId}/clear`);
    },
    [deviceId],
  );
  const onChannelAdd = useCallback(
    (channelId: number, multiplier: number) => {
      devicePostJsonEmpty(deviceId, `/channels/${channelId}/add`, multiplier);
    },
    [deviceId],
  );
  const onChannelMoveFront = useCallback(
    (channelId: number) => {
      devicePostEmpty(deviceId, `/channels/${channelId}/move-front`);
    },
    [deviceId],
  );
  const onChannelMoveBack = useCallback(
    (channelId: number) => {
      devicePostEmpty(deviceId, `/channels/${channelId}/move-back`);
    },
    [deviceId],
  );

  if (deviceSummary === undefined) {
    return null;
  }

  return (
    <Wrapper>
      <Section>
        <ButtonGroup>
          <Button active={deviceSummaryStateIsEnabled(deviceSummary.state)} onClick={onDeviceEnable}>
            Enable
          </Button>
          <Button active={deviceSummaryStateIsPaused(deviceSummary.state)} onClick={onDevicePause}>
            Pause
          </Button>
          <Button active={deviceSummaryStateIsDisabled(deviceSummary.state)} onClick={onDeviceDisable}>
            Disable
          </Button>
        </ButtonGroup>

        {deviceSummaryStateIsPaused(deviceSummary.state) || deviceSummaryStateIsEnabled(deviceSummary.state) ? (
          <ButtonGroup>
            <Button onClick={() => onChannelsAllAdd(1.0)}>+1</Button>
            <Button onClick={() => onChannelsAllAdd(0.5)}>+1/2</Button>
            <Button onClick={() => onChannelsAllAdd(0.25)}>+1/4</Button>
            <Button onClick={onChannelsAllClear}>Clear</Button>
          </ButtonGroup>
        ) : null}

        {deviceSummaryStateIsEnabled(deviceSummary.state) ? (
          <Chip type={ChipType.INFO} enabled={deviceSummary.state.power > 0}>
            Power usage: {deviceSummary.state.power.toFixed(2)} / {deviceSummary.configuration.power_max.toFixed(2)}
          </Chip>
        ) : null}
      </Section>

      <ChannelSections>
        {deviceSummary.configuration.channels
          .map((channelConfiguration, channelId) => ({
            channelConfiguration,
            channelState: deviceSummary.state.channels[channelId],
          }))
          .map(({ channelConfiguration, channelState }, channelId) => (
            <ChannelSection key={channelId}>
              <ChannelSectionTitle>{channelConfiguration.name}</ChannelSectionTitle>
              <Section>
                {/* Enable/Disable */}
                <ButtonGroup>
                  <Button
                    active={
                      (deviceSummaryStateIsDisabled(deviceSummary.state) &&
                        deviceSummaryDeviceStateDisabledChannelStateIsEnabled(
                          channelState as DeviceSummaryDeviceStateDisabledChannelState,
                        )) ||
                      (deviceSummaryStateIsPaused(deviceSummary.state) &&
                        deviceSummaryDeviceStatePausedChannelStateIsEnabled(
                          channelState as DeviceSummaryDeviceStatePausedChannelState,
                        )) ||
                      (deviceSummaryStateIsEnabled(deviceSummary.state) &&
                        (deviceSummaryDeviceStateEnabledChannelStateIsEnabledQueued(
                          channelState as DeviceSummaryDeviceStateEnabledChannelState,
                        ) ||
                          deviceSummaryDeviceStateEnabledChannelStateIsEnabledActive(
                            channelState as DeviceSummaryDeviceStateEnabledChannelState,
                          )))
                    }
                    onClick={() => onChannelEnable(channelId)}
                  >
                    Enable
                  </Button>
                  <Button
                    active={
                      (deviceSummaryStateIsDisabled(deviceSummary.state) &&
                        deviceSummaryDeviceStateDisabledChannelStateIsPaused(
                          channelState as DeviceSummaryDeviceStateDisabledChannelState,
                        )) ||
                      (deviceSummaryStateIsPaused(deviceSummary.state) &&
                        deviceSummaryDeviceStatePausedChannelStateIsPaused(
                          channelState as DeviceSummaryDeviceStatePausedChannelState,
                        )) ||
                      (deviceSummaryStateIsEnabled(deviceSummary.state) &&
                        deviceSummaryDeviceStateEnabledChannelStateIsPaused(
                          channelState as DeviceSummaryDeviceStateEnabledChannelState,
                        ))
                    }
                    onClick={() => onChannelPause(channelId)}
                  >
                    Pause
                  </Button>
                  <Button
                    active={
                      (deviceSummaryStateIsDisabled(deviceSummary.state) &&
                        deviceSummaryDeviceStateDisabledChannelStateIsDisabled(
                          channelState as DeviceSummaryDeviceStateDisabledChannelState,
                        )) ||
                      (deviceSummaryStateIsPaused(deviceSummary.state) &&
                        deviceSummaryDeviceStatePausedChannelStateIsDisabled(
                          channelState as DeviceSummaryDeviceStatePausedChannelState,
                        )) ||
                      (deviceSummaryStateIsEnabled(deviceSummary.state) &&
                        deviceSummaryDeviceStateEnabledChannelStateIsDisabled(
                          channelState as DeviceSummaryDeviceStateEnabledChannelState,
                        ))
                    }
                    onClick={() => onChannelDisable(channelId)}
                  >
                    Disable
                  </Button>
                </ButtonGroup>

                {/* Add time / clear */}
                <>
                  {(deviceSummaryStateIsPaused(deviceSummary.state) &&
                    (deviceSummaryDeviceStatePausedChannelStateIsPaused(
                      channelState as DeviceSummaryDeviceStatePausedChannelState,
                    ) ||
                      deviceSummaryDeviceStatePausedChannelStateIsEnabled(
                        channelState as DeviceSummaryDeviceStatePausedChannelState,
                      ))) ||
                  (deviceSummaryStateIsEnabled(deviceSummary.state) &&
                    (deviceSummaryDeviceStateEnabledChannelStateIsPaused(
                      channelState as DeviceSummaryDeviceStateEnabledChannelState,
                    ) ||
                      deviceSummaryDeviceStateEnabledChannelStateIsEnabledQueued(
                        channelState as DeviceSummaryDeviceStateEnabledChannelState,
                      ) ||
                      deviceSummaryDeviceStateEnabledChannelStateIsEnabledActive(
                        channelState as DeviceSummaryDeviceStateEnabledChannelState,
                      ))) ? (
                    <ButtonGroup>
                      <Button onClick={() => onChannelAdd(channelId, 1.0)}>+1</Button>
                      <Button onClick={() => onChannelAdd(channelId, 0.5)}>+1/2</Button>
                      <Button onClick={() => onChannelAdd(channelId, 0.25)}>+1/4</Button>
                      <Button onClick={() => onChannelClear(channelId)}>Clear</Button>
                    </ButtonGroup>
                  ) : null}
                </>

                {/* Move front / back */}
                <>
                  {deviceSummaryStateIsEnabled(deviceSummary.state) &&
                  (deviceSummaryDeviceStateEnabledChannelStateIsEnabledQueued(
                    channelState as DeviceSummaryDeviceStateEnabledChannelState,
                  ) ||
                    deviceSummaryDeviceStateEnabledChannelStateIsEnabledActive(
                      channelState as DeviceSummaryDeviceStateEnabledChannelState,
                    )) ? (
                    <ButtonGroup>
                      <ButtonGroup>
                        <Button onClick={() => onChannelMoveFront(channelId)}>Move front</Button>
                        <Button onClick={() => onChannelMoveBack(channelId)}>Move back</Button>
                      </ButtonGroup>
                    </ButtonGroup>
                  ) : null}
                </>
              </Section>
              <Section>
                {/* Order index */}
                <>
                  {deviceSummaryStateIsEnabled(deviceSummary.state) &&
                  deviceSummaryDeviceStateEnabledChannelStateIsEnabledQueued(
                    channelState as DeviceSummaryDeviceStateEnabledChannelState,
                  ) &&
                  (channelState as DeviceSummaryDeviceStateEnabledChannelStateEnabledQueued).queue_position !== null ? (
                    <Chip type={ChipType.INFO} enabled>
                      Queue position:{" "}
                      {(channelState as DeviceSummaryDeviceStateEnabledChannelStateEnabledQueued).queue_position}
                    </Chip>
                  ) : null}
                </>

                {/* Round time */}
                <>
                  {deviceSummaryStateIsEnabled(deviceSummary.state) &&
                  deviceSummaryDeviceStateEnabledChannelStateIsEnabledActive(
                    channelState as DeviceSummaryDeviceStateEnabledChannelState,
                  ) ? (
                    <Chip type={ChipType.OK} enabled>
                      Round time:{" "}
                      {(channelState as DeviceSummaryDeviceStateEnabledChannelStateEnabledActive).round_seconds}
                    </Chip>
                  ) : null}
                </>

                {/* Queue time */}
                <>
                  {(deviceSummaryStateIsPaused(deviceSummary.state) &&
                    (deviceSummaryDeviceStatePausedChannelStateIsPaused(
                      channelState as DeviceSummaryDeviceStatePausedChannelState,
                    ) ||
                      deviceSummaryDeviceStatePausedChannelStateIsEnabled(
                        channelState as DeviceSummaryDeviceStatePausedChannelState,
                      ))) ||
                  (deviceSummaryStateIsEnabled(deviceSummary.state) &&
                    (deviceSummaryDeviceStateEnabledChannelStateIsPaused(
                      channelState as DeviceSummaryDeviceStateEnabledChannelState,
                    ) ||
                      deviceSummaryDeviceStateEnabledChannelStateIsEnabledQueued(
                        channelState as DeviceSummaryDeviceStateEnabledChannelState,
                      ) ||
                      deviceSummaryDeviceStateEnabledChannelStateIsEnabledActive(
                        channelState as DeviceSummaryDeviceStateEnabledChannelState,
                      ))) ? (
                    <Chip
                      type={ChipType.OK}
                      enabled={
                        deviceSummaryStateIsEnabled(deviceSummary.state) &&
                        (deviceSummaryDeviceStateEnabledChannelStateIsEnabledQueued(
                          channelState as DeviceSummaryDeviceStateEnabledChannelState,
                        ) ||
                          deviceSummaryDeviceStateEnabledChannelStateIsEnabledActive(
                            channelState as DeviceSummaryDeviceStateEnabledChannelState,
                          )) &&
                        // eslint-disable-next-line @typescript-eslint/no-explicit-any
                        ((channelState as any).queue_seconds as number) >= channelConfiguration.round_min_seconds
                      }
                    >
                      {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
                      Queue time: {(channelState as any).queue_seconds as number}
                    </Chip>
                  ) : null}
                </>

                {/* Base time */}
                <>
                  <Chip type={ChipType.INFO} enabled={false}>
                    Base time: {channelConfiguration.base_time_seconds}
                  </Chip>
                </>
                {/* Power required */}
                <>
                  <Chip
                    type={ChipType.INFO}
                    enabled={
                      deviceSummaryStateIsEnabled(deviceSummary.state) &&
                      deviceSummaryDeviceStateEnabledChannelStateIsEnabledActive(
                        channelState as DeviceSummaryDeviceStateEnabledChannelState,
                      )
                    }
                  >
                    Power required: {channelConfiguration.power_required.toFixed(2)}
                  </Chip>
                </>

                {/* Round min/max */}
                <>
                  <Chip
                    type={ChipType.INFO}
                    enabled={
                      (deviceSummaryStateIsPaused(deviceSummary.state) &&
                        deviceSummaryDeviceStatePausedChannelStateIsEnabled(
                          channelState as DeviceSummaryDeviceStatePausedChannelState,
                        )) ||
                      (deviceSummaryStateIsEnabled(deviceSummary.state) &&
                        deviceSummaryDeviceStateEnabledChannelStateIsEnabledQueued(
                          channelState as DeviceSummaryDeviceStateEnabledChannelState,
                        )) ||
                      deviceSummaryDeviceStateEnabledChannelStateIsEnabledActive(
                        channelState as DeviceSummaryDeviceStateEnabledChannelState,
                      )
                    }
                  >
                    Round time min/max: {channelConfiguration.round_min_seconds}/
                    {channelConfiguration.round_max_seconds}
                  </Chip>
                </>
              </Section>
            </ChannelSection>
          ))}
      </ChannelSections>
    </Wrapper>
  );
};
export default SummaryManaged;

const Wrapper = styled.div``;
const Section = styled.div`
  display: flex;
  flex-wrap: wrap;
  align-items: center;

  & > * {
    margin: 0.25rem;
  }
`;
const ChannelSection = styled.div`
  margin-left: 2rem;
  padding: 0.5rem 1rem;
`;
const ChannelSections = styled.div`
  & > ${ChannelSection} {
    border-bottom: solid 1px ${Colors.GREY_LIGHTEST};
    &:last-child {
      border-bottom: none;
    }
  }
`;
const ChannelSectionTitle = styled.p`
  font-size: 1.25rem;
  font-weight: bold;
`;
