import { ButtonGroup, ButtonLink } from "components/common/Button";
import { Chip, ChipsGroup, ChipType } from "components/common/Chips";
import { Line } from "components/common/Line";
import {
  SnapshotDeviceInner,
  SnapshotDeviceInnerNone,
} from "components/devices/soft/surveillance/ipc/SnapshotDeviceInner";
import React from "react";
import styled from "styled-components";

export type Data = DataInitializing | DataRunning | DataError;
export interface DataInitializing {
  state: "Initializing";
}
export function dataIsInitializing(data: Data): data is DataInitializing {
  return data.state === "Initializing";
}
export interface DataRunning {
  state: "Running";
  snapshot_updated: string | null;
  rtsp_urls: DataRtspUrls;
  events: DataEvents;
}
export function dataIsRunning(data: Data): data is DataRunning {
  return data.state === "Running";
}
export interface DataError {
  state: "Error";
}
export function dataIsError(data: Data): data is DataError {
  return data.state === "Error";
}

export interface DataRtspUrls {
  main: string;
  sub1: string;
  sub2: string;
}
export interface DataEvents {
  video_blind: boolean;
  scene_change: boolean;
  video_motion: boolean;
  audio_mutation: boolean;
  smart_motion_human: boolean;
  smart_motion_vehicle: boolean;
}

const Component: React.VFC<{
  data: Data | undefined;
  snapshotBaseUrl: string | undefined;
}> = (props) => {
  const { data, snapshotBaseUrl } = props;

  return (
    <Wrapper>
      <Header>
        <State>
          {data !== undefined && dataIsInitializing(data) ? (
            <Chip type={ChipType.ERROR} enabled={true}>
              Initializing
            </Chip>
          ) : null}
          {data !== undefined && dataIsRunning(data) ? (
            <Chip type={ChipType.OK} enabled={true}>
              Running
            </Chip>
          ) : null}
          {data !== undefined && dataIsError(data) ? (
            <Chip type={ChipType.ERROR} enabled={true}>
              Error
            </Chip>
          ) : null}
        </State>
        <Events>
          {data !== undefined && dataIsRunning(data) ? (
            <ChipsGroup>
              <Chip type={ChipType.ERROR} enabled={data.events.video_blind}>
                Video Blind
              </Chip>
              <Chip type={ChipType.WARNING} enabled={data.events.scene_change}>
                Scene Changed
              </Chip>
              <Chip type={ChipType.INFO} enabled={data.events.video_motion}>
                Video Motion
              </Chip>
              <Chip type={ChipType.INFO} enabled={data.events.audio_mutation}>
                Audio Mutation
              </Chip>
              <Chip type={ChipType.INFO} enabled={data.events.smart_motion_human}>
                Human Motion
              </Chip>
              <Chip type={ChipType.INFO} enabled={data.events.smart_motion_vehicle}>
                Vehicle Motion
              </Chip>
            </ChipsGroup>
          ) : null}
        </Events>
      </Header>
      <Line />
      <Snapshot>
        {snapshotBaseUrl !== undefined &&
        data !== undefined &&
        dataIsRunning(data) &&
        data.snapshot_updated !== null ? (
          <SnapshotDeviceInner baseUrl={snapshotBaseUrl} lastUpdated={new Date(data.snapshot_updated)} />
        ) : (
          <SnapshotDeviceInnerNone />
        )}
      </Snapshot>
      <Line />
      <RtspUrls>
        {data !== undefined && dataIsRunning(data) ? (
          <ButtonGroup>
            <ButtonLink targetBlank href={data.rtsp_urls.main}>
              Main Stream
            </ButtonLink>
            <ButtonLink targetBlank href={data.rtsp_urls.sub1}>
              Sub Stream 1
            </ButtonLink>
            <ButtonLink targetBlank href={data.rtsp_urls.sub2}>
              Sub Stream 2
            </ButtonLink>
          </ButtonGroup>
        ) : null}
      </RtspUrls>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;
const Header = styled.div`
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(10rem, auto));
  grid-auto-rows: auto;
  grid-gap: 0.5rem;
  align-items: center;
  justify-content: space-between;
`;
const State = styled.div``;
const Events = styled.div``;

const Snapshot = styled.div`
  text-align: center;
`;

const RtspUrls = styled.div``;
