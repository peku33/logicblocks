import { ButtonActionAsync, ButtonGroup } from "@/components/common/Button";
import { Chip, ChipType } from "@/components/common/Chips";
import Colors from "@/components/common/Colors";
import styled from "styled-components";

export interface Data {
  configuration: DataConfiguration;
  state: DataState;
}

export interface DataConfiguration {
  channels: DataChannelConfiguration[];
  power_max: number;
}

export interface DataChannelConfiguration {
  name: string;

  base_time_seconds: number;
  power_required: number;

  round_min_seconds: number;
  round_max_seconds: number;
}

export type DataState = DataStateDisabled | DataStatePaused | DataStateEnabled;
export interface DataStateDisabled {
  state: "Disabled";
  channels: DataDeviceStateDisabledChannelState[];
}
export function dataStateIsDisabled(dataState: DataState): dataState is DataStateDisabled {
  return dataState.state === "Disabled";
}
export interface DataStatePaused {
  state: "Paused";
  channels: DataDeviceStatePausedChannelState[];
}
export function dataStateIsPaused(dataState: DataState): dataState is DataStatePaused {
  return dataState.state === "Paused";
}
export interface DataStateEnabled {
  state: "Enabled";
  channels: DataDeviceStateEnabledChannelState[];
  power: number;
}
export function dataStateIsEnabled(dataState: DataState): dataState is DataStateEnabled {
  return dataState.state === "Enabled";
}

// DataDeviceStateDisabledChannelState
export type DataDeviceStateDisabledChannelState =
  | DataDeviceStateDisabledChannelStateDisabled
  | DataDeviceStateDisabledChannelStatePaused
  | DataDeviceStateDisabledChannelStateEnabled;
export interface DataDeviceStateDisabledChannelStateDisabled {
  state: "Disabled";
}
export function dataDeviceStateDisabledChannelStateIsDisabled(
  dataDeviceStateDisabledChannelState: DataDeviceStateDisabledChannelState,
): dataDeviceStateDisabledChannelState is DataDeviceStateDisabledChannelStateDisabled {
  return dataDeviceStateDisabledChannelState.state === "Disabled";
}
export interface DataDeviceStateDisabledChannelStatePaused {
  state: "Paused";
}
export function dataDeviceStateDisabledChannelStateIsPaused(
  dataDeviceStateDisabledChannelState: DataDeviceStateDisabledChannelState,
): dataDeviceStateDisabledChannelState is DataDeviceStateDisabledChannelStatePaused {
  return dataDeviceStateDisabledChannelState.state === "Paused";
}
export interface DataDeviceStateDisabledChannelStateEnabled {
  state: "Enabled";
}
export function dataDeviceStateDisabledChannelStateIsEnabled(
  dataDeviceStateDisabledChannelState: DataDeviceStateDisabledChannelState,
): dataDeviceStateDisabledChannelState is DataDeviceStateDisabledChannelStateEnabled {
  return dataDeviceStateDisabledChannelState.state === "Enabled";
}

// DataDeviceStatePausedChannelState
export type DataDeviceStatePausedChannelState =
  | DataDeviceStatePausedChannelStateDisabled
  | DataDeviceStatePausedChannelStatePaused
  | DataDeviceStatePausedChannelStateEnabled;
export interface DataDeviceStatePausedChannelStateDisabled {
  state: "Disabled";
}
export function dataDeviceStatePausedChannelStateIsDisabled(
  dataDeviceStatePausedChannelState: DataDeviceStatePausedChannelState,
): dataDeviceStatePausedChannelState is DataDeviceStatePausedChannelStateDisabled {
  return dataDeviceStatePausedChannelState.state === "Disabled";
}
export interface DataDeviceStatePausedChannelStatePaused {
  state: "Paused";
  queue_seconds: number;
}
export function dataDeviceStatePausedChannelStateIsPaused(
  dataDeviceStatePausedChannelState: DataDeviceStatePausedChannelState,
): dataDeviceStatePausedChannelState is DataDeviceStatePausedChannelStatePaused {
  return dataDeviceStatePausedChannelState.state === "Paused";
}
export interface DataDeviceStatePausedChannelStateEnabled {
  state: "Enabled";
  queue_seconds: number;
}
export function dataDeviceStatePausedChannelStateIsEnabled(
  dataDeviceStatePausedChannelState: DataDeviceStatePausedChannelState,
): dataDeviceStatePausedChannelState is DataDeviceStatePausedChannelStateEnabled {
  return dataDeviceStatePausedChannelState.state === "Enabled";
}

// DataDeviceStateEnabledChannelState
export type DataDeviceStateEnabledChannelState =
  | DataDeviceStateEnabledChannelStateDisabled
  | DataDeviceStateEnabledChannelStatePaused
  | DataDeviceStateEnabledChannelStateEnabledQueued
  | DataDeviceStateEnabledChannelStateEnabledActive;
export interface DataDeviceStateEnabledChannelStateDisabled {
  state: "Disabled";
}
export function dataDeviceStateEnabledChannelStateIsDisabled(
  dataDeviceStateEnabledChannelState: DataDeviceStateEnabledChannelState,
): dataDeviceStateEnabledChannelState is DataDeviceStateEnabledChannelStateDisabled {
  return dataDeviceStateEnabledChannelState.state === "Disabled";
}
export interface DataDeviceStateEnabledChannelStatePaused {
  state: "Paused";
  queue_seconds: number;
}
export function dataDeviceStateEnabledChannelStateIsPaused(
  dataDeviceStateEnabledChannelState: DataDeviceStateEnabledChannelState,
): dataDeviceStateEnabledChannelState is DataDeviceStateEnabledChannelStatePaused {
  return dataDeviceStateEnabledChannelState.state === "Paused";
}
export interface DataDeviceStateEnabledChannelStateEnabledQueued {
  state: "EnabledQueued";
  queue_seconds: number;
  queue_position: number | null;
}
export function dataDeviceStateEnabledChannelStateIsEnabledQueued(
  dataDeviceStateEnabledChannelState: DataDeviceStateEnabledChannelState,
): dataDeviceStateEnabledChannelState is DataDeviceStateEnabledChannelStateEnabledQueued {
  return dataDeviceStateEnabledChannelState.state === "EnabledQueued";
}
export interface DataDeviceStateEnabledChannelStateEnabledActive {
  state: "EnabledActive";
  queue_seconds: number;
  round_seconds: number;
}
export function dataDeviceStateEnabledChannelStateIsEnabledActive(
  dataDeviceStateEnabledChannelState: DataDeviceStateEnabledChannelState,
): dataDeviceStateEnabledChannelState is DataDeviceStateEnabledChannelStateEnabledActive {
  return dataDeviceStateEnabledChannelState.state === "EnabledActive";
}

const Component: React.FC<{
  data: Data | undefined;
  onDeviceDisable: () => Promise<void>;
  onDevicePause: () => Promise<void>;
  onDeviceEnable: () => Promise<void>;
  onChannelsAllClear: () => Promise<void>;
  onChannelsAllAdd: (multiplier: number) => Promise<void>;
  onChannelDisable: (channelId: number) => Promise<void>;
  onChannelPause: (channelId: number) => Promise<void>;
  onChannelEnable: (channelId: number) => Promise<void>;
  onChannelClear: (channelId: number) => Promise<void>;
  onChannelAdd: (channelId: number, multiplier: number) => Promise<void>;
  onChannelMoveFront: (channelId: number) => Promise<void>;
  onChannelMoveBack: (channelId: number) => Promise<void>;
}> = (props) => {
  const {
    data,
    onDeviceDisable,
    onDevicePause,
    onDeviceEnable,
    onChannelsAllClear,
    onChannelsAllAdd,
    onChannelDisable,
    onChannelPause,
    onChannelEnable,
    onChannelClear,
    onChannelAdd,
    onChannelMoveFront,
    onChannelMoveBack,
  } = props;

  if (data === undefined) {
    return null;
  }

  return (
    <Wrapper>
      <Section>
        <ButtonGroup>
          <ButtonActionAsync active={dataStateIsDisabled(data.state)} onClick={onDeviceDisable}>
            Disable
          </ButtonActionAsync>
          <ButtonActionAsync active={dataStateIsPaused(data.state)} onClick={onDevicePause}>
            Pause
          </ButtonActionAsync>
          <ButtonActionAsync active={dataStateIsEnabled(data.state)} onClick={onDeviceEnable}>
            Enable
          </ButtonActionAsync>
        </ButtonGroup>

        {dataStateIsPaused(data.state) || dataStateIsEnabled(data.state) ? (
          <ButtonGroup>
            <ButtonActionAsync active={false} onClick={onChannelsAllClear}>
              Clear
            </ButtonActionAsync>
            <ButtonActionAsync
              active={false}
              onClick={async () => {
                await onChannelsAllAdd(0.25);
              }}
            >
              +1/4
            </ButtonActionAsync>
            <ButtonActionAsync
              active={false}
              onClick={async () => {
                await onChannelsAllAdd(0.5);
              }}
            >
              +1/2
            </ButtonActionAsync>
            <ButtonActionAsync
              active={false}
              onClick={async () => {
                await onChannelsAllAdd(1.0);
              }}
            >
              +1
            </ButtonActionAsync>
          </ButtonGroup>
        ) : null}

        {dataStateIsEnabled(data.state) ? (
          <Chip type={ChipType.INFO} enabled={data.state.power > 0}>
            Power usage: {data.state.power.toFixed(2)} / {data.configuration.power_max.toFixed(2)}
          </Chip>
        ) : null}
      </Section>

      <ChannelSections>
        {data.configuration.channels
          .map((channelConfiguration, channelId) => ({
            channelConfiguration,
            channelState: data.state.channels[channelId],
          }))
          .map(({ channelConfiguration, channelState }, channelId) => (
            <ChannelSection key={channelId}>
              <ChannelSectionTitle>{channelConfiguration.name}</ChannelSectionTitle>
              <Section>
                {/* Enable/Disable */}
                <ButtonGroup>
                  <ButtonActionAsync
                    active={
                      (dataStateIsDisabled(data.state) &&
                        dataDeviceStateDisabledChannelStateIsDisabled(
                          channelState as DataDeviceStateDisabledChannelState,
                        )) ||
                      (dataStateIsPaused(data.state) &&
                        dataDeviceStatePausedChannelStateIsDisabled(
                          channelState as DataDeviceStatePausedChannelState,
                        )) ||
                      (dataStateIsEnabled(data.state) &&
                        dataDeviceStateEnabledChannelStateIsDisabled(
                          channelState as DataDeviceStateEnabledChannelState,
                        ))
                    }
                    onClick={async () => {
                      await onChannelDisable(channelId);
                    }}
                  >
                    Disable
                  </ButtonActionAsync>
                  <ButtonActionAsync
                    active={
                      (dataStateIsDisabled(data.state) &&
                        dataDeviceStateDisabledChannelStateIsPaused(
                          channelState as DataDeviceStateDisabledChannelState,
                        )) ||
                      (dataStateIsPaused(data.state) &&
                        dataDeviceStatePausedChannelStateIsPaused(channelState as DataDeviceStatePausedChannelState)) ||
                      (dataStateIsEnabled(data.state) &&
                        dataDeviceStateEnabledChannelStateIsPaused(channelState as DataDeviceStateEnabledChannelState))
                    }
                    onClick={async () => {
                      await onChannelPause(channelId);
                    }}
                  >
                    Pause
                  </ButtonActionAsync>
                  <ButtonActionAsync
                    active={
                      (dataStateIsDisabled(data.state) &&
                        dataDeviceStateDisabledChannelStateIsEnabled(
                          channelState as DataDeviceStateDisabledChannelState,
                        )) ||
                      (dataStateIsPaused(data.state) &&
                        dataDeviceStatePausedChannelStateIsEnabled(
                          channelState as DataDeviceStatePausedChannelState,
                        )) ||
                      (dataStateIsEnabled(data.state) &&
                        (dataDeviceStateEnabledChannelStateIsEnabledQueued(
                          channelState as DataDeviceStateEnabledChannelState,
                        ) ||
                          dataDeviceStateEnabledChannelStateIsEnabledActive(
                            channelState as DataDeviceStateEnabledChannelState,
                          )))
                    }
                    onClick={async () => {
                      await onChannelEnable(channelId);
                    }}
                  >
                    Enable
                  </ButtonActionAsync>
                </ButtonGroup>

                {/* Add time / clear */}
                <>
                  {(dataStateIsPaused(data.state) &&
                    (dataDeviceStatePausedChannelStateIsPaused(channelState as DataDeviceStatePausedChannelState) ||
                      dataDeviceStatePausedChannelStateIsEnabled(channelState as DataDeviceStatePausedChannelState))) ||
                  (dataStateIsEnabled(data.state) &&
                    (dataDeviceStateEnabledChannelStateIsPaused(channelState as DataDeviceStateEnabledChannelState) ||
                      dataDeviceStateEnabledChannelStateIsEnabledQueued(
                        channelState as DataDeviceStateEnabledChannelState,
                      ) ||
                      dataDeviceStateEnabledChannelStateIsEnabledActive(
                        channelState as DataDeviceStateEnabledChannelState,
                      ))) ? (
                    <ButtonGroup>
                      <ButtonActionAsync
                        active={false}
                        onClick={async () => {
                          await onChannelClear(channelId);
                        }}
                      >
                        Clear
                      </ButtonActionAsync>
                      <ButtonActionAsync
                        active={false}
                        onClick={async () => {
                          await onChannelAdd(channelId, 0.25);
                        }}
                      >
                        +1/4
                      </ButtonActionAsync>
                      <ButtonActionAsync
                        active={false}
                        onClick={async () => {
                          await onChannelAdd(channelId, 0.5);
                        }}
                      >
                        +1/2
                      </ButtonActionAsync>
                      <ButtonActionAsync
                        active={false}
                        onClick={async () => {
                          await onChannelAdd(channelId, 1.0);
                        }}
                      >
                        +1
                      </ButtonActionAsync>
                    </ButtonGroup>
                  ) : null}
                </>

                {/* Move front / back */}
                <>
                  {dataStateIsEnabled(data.state) &&
                  (dataDeviceStateEnabledChannelStateIsEnabledQueued(
                    channelState as DataDeviceStateEnabledChannelState,
                  ) ||
                    dataDeviceStateEnabledChannelStateIsEnabledActive(
                      channelState as DataDeviceStateEnabledChannelState,
                    )) ? (
                    <ButtonGroup>
                      <ButtonGroup>
                        <ButtonActionAsync
                          active={false}
                          onClick={async () => {
                            await onChannelMoveBack(channelId);
                          }}
                        >
                          Move back
                        </ButtonActionAsync>
                        <ButtonActionAsync
                          active={false}
                          onClick={async () => {
                            await onChannelMoveFront(channelId);
                          }}
                        >
                          Move front
                        </ButtonActionAsync>
                      </ButtonGroup>
                    </ButtonGroup>
                  ) : null}
                </>
              </Section>
              <Section>
                {/* Order index */}
                <>
                  {dataStateIsEnabled(data.state) &&
                  dataDeviceStateEnabledChannelStateIsEnabledQueued(
                    channelState as DataDeviceStateEnabledChannelState,
                  ) &&
                  (channelState as DataDeviceStateEnabledChannelStateEnabledQueued).queue_position !== null ? (
                    <Chip type={ChipType.INFO} enabled>
                      Queue position: {(channelState as DataDeviceStateEnabledChannelStateEnabledQueued).queue_position}
                    </Chip>
                  ) : null}
                </>

                {/* Round time */}
                <>
                  {dataStateIsEnabled(data.state) &&
                  dataDeviceStateEnabledChannelStateIsEnabledActive(
                    channelState as DataDeviceStateEnabledChannelState,
                  ) ? (
                    <Chip type={ChipType.OK} enabled>
                      Round time: {(channelState as DataDeviceStateEnabledChannelStateEnabledActive).round_seconds}
                    </Chip>
                  ) : null}
                </>

                {/* Queue time */}
                <>
                  {(dataStateIsPaused(data.state) &&
                    (dataDeviceStatePausedChannelStateIsPaused(channelState as DataDeviceStatePausedChannelState) ||
                      dataDeviceStatePausedChannelStateIsEnabled(channelState as DataDeviceStatePausedChannelState))) ||
                  (dataStateIsEnabled(data.state) &&
                    (dataDeviceStateEnabledChannelStateIsPaused(channelState as DataDeviceStateEnabledChannelState) ||
                      dataDeviceStateEnabledChannelStateIsEnabledQueued(
                        channelState as DataDeviceStateEnabledChannelState,
                      ) ||
                      dataDeviceStateEnabledChannelStateIsEnabledActive(
                        channelState as DataDeviceStateEnabledChannelState,
                      ))) ? (
                    <Chip
                      type={ChipType.OK}
                      enabled={
                        dataStateIsEnabled(data.state) &&
                        (dataDeviceStateEnabledChannelStateIsEnabledQueued(
                          channelState as DataDeviceStateEnabledChannelState,
                        ) ||
                          dataDeviceStateEnabledChannelStateIsEnabledActive(
                            channelState as DataDeviceStateEnabledChannelState,
                          )) &&
                        // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-explicit-any
                        ((channelState as any).queue_seconds as number) >= channelConfiguration.round_min_seconds
                      }
                    >
                      {/* eslint-disable-next-line @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-explicit-any */}
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
                      dataStateIsEnabled(data.state) &&
                      dataDeviceStateEnabledChannelStateIsEnabledActive(
                        channelState as DataDeviceStateEnabledChannelState,
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
                      (dataStateIsPaused(data.state) &&
                        dataDeviceStatePausedChannelStateIsEnabled(
                          channelState as DataDeviceStatePausedChannelState,
                        )) ||
                      (dataStateIsEnabled(data.state) &&
                        dataDeviceStateEnabledChannelStateIsEnabledQueued(
                          channelState as DataDeviceStateEnabledChannelState,
                        )) ||
                      dataDeviceStateEnabledChannelStateIsEnabledActive(
                        channelState as DataDeviceStateEnabledChannelState,
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
export default Component;

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
  margin-left: 0.5rem;
  padding: 0.25rem 0.5rem;
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
  font-size: large;
  font-weight: bold;
`;
