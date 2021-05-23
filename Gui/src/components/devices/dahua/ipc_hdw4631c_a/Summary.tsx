import { ButtonGroup, ButtonLink } from "components/common/Button";
import { Chip, ChipsGroup, ChipType } from "components/common/Chips";
import Colors from "components/common/Colors";
import {
  SnapshotDeviceInner,
  SnapshotDeviceInnerNone,
} from "components/devices/soft/surveillance/ipc/SnapshotDeviceInner";
import { urlBuild } from "lib/Api";
import React from "react";
import { deviceEndpointBuild, useDeviceSummary } from "services/LogicDevicesRunner";
import styled from "styled-components";

interface DeviceSummaryRtspUrls {
  main: string;
  sub1: string;
  sub2: string;
}
interface DeviceSummaryEvents {
  video_blind: boolean;
  scene_change: boolean;
  video_motion: boolean;
  audio_mutation: boolean;
}

interface DeviceSummaryInitializing {
  state: "Initializing";
}
interface DeviceSummaryRunning {
  state: "Running";
  snapshot_updated: string | null;
  rtsp_urls: DeviceSummaryRtspUrls;
  events: DeviceSummaryEvents;
}
interface DeviceSummaryError {
  state: "Error";
}
type DeviceSummary = DeviceSummaryInitializing | DeviceSummaryRunning | DeviceSummaryError;
function isDeviceSummaryInitializing(deviceSummary: DeviceSummary): deviceSummary is DeviceSummaryInitializing {
  return deviceSummary.state === "Initializing";
}
function isDeviceSummaryRuning(deviceSummary: DeviceSummary): deviceSummary is DeviceSummaryRunning {
  return deviceSummary.state === "Running";
}
function isDeviceSummaryError(deviceSummary: DeviceSummary): deviceSummary is DeviceSummaryError {
  return deviceSummary.state === "Error";
}

const Summary: React.FC<{
  deviceId: number;
  deviceClass: string;
}> = (props) => {
  const { deviceId } = props;

  const deviceSummary = useDeviceSummary<DeviceSummary>(deviceId);

  return (
    <Wrapper>
      <Header>
        <State>
          {deviceSummary !== undefined && isDeviceSummaryInitializing(deviceSummary) ? (
            <Chip type={ChipType.ERROR} enabled={true}>
              Initializing
            </Chip>
          ) : null}
          {deviceSummary !== undefined && isDeviceSummaryRuning(deviceSummary) ? (
            <Chip type={ChipType.OK} enabled={true}>
              Running
            </Chip>
          ) : null}
          {deviceSummary !== undefined && isDeviceSummaryError(deviceSummary) ? (
            <Chip type={ChipType.ERROR} enabled={true}>
              Error
            </Chip>
          ) : null}
        </State>
        <Events>
          {deviceSummary !== undefined && isDeviceSummaryRuning(deviceSummary) ? (
            <ChipsGroup>
              <Chip type={ChipType.ERROR} enabled={deviceSummary.events.video_blind}>
                Video Blind
              </Chip>
              <Chip type={ChipType.WARNING} enabled={deviceSummary.events.scene_change}>
                Scene Changed
              </Chip>
              <Chip type={ChipType.INFO} enabled={deviceSummary.events.video_motion}>
                Video Motion
              </Chip>
              <Chip type={ChipType.INFO} enabled={deviceSummary.events.audio_mutation}>
                Audio Mutation
              </Chip>
            </ChipsGroup>
          ) : null}
        </Events>
      </Header>
      <Snapshot>
        {deviceSummary !== undefined &&
        isDeviceSummaryRuning(deviceSummary) &&
        deviceSummary.snapshot_updated !== null ? (
          <SnapshotDeviceInner
            baseUrl={urlBuild(deviceEndpointBuild(deviceId, "/snapshot"))}
            lastUpdated={new Date(deviceSummary.snapshot_updated)}
          />
        ) : (
          <SnapshotDeviceInnerNone />
        )}
      </Snapshot>
      <RtspUrls>
        {deviceSummary !== undefined && isDeviceSummaryRuning(deviceSummary) ? (
          <ButtonGroup>
            <ButtonLink target="_blank" href={deviceSummary.rtsp_urls.main}>
              Main Stream
            </ButtonLink>
            <ButtonLink target="_blank" href={deviceSummary.rtsp_urls.sub1}>
              Sub Stream 1
            </ButtonLink>
            <ButtonLink target="_blank" href={deviceSummary.rtsp_urls.sub2}>
              Sub Stream 2
            </ButtonLink>
          </ButtonGroup>
        ) : null}
      </RtspUrls>
    </Wrapper>
  );
};

export default Summary;

const Wrapper = styled.div``;

const Header = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;

  margin-bottom: 0.5rem;
  padding-bottom: 0.5rem;

  border-bottom: solid 1px ${Colors.GREY_LIGHTEST};
`;
const State = styled.div``;
const Events = styled.div``;

const Snapshot = styled.div`
  text-align: center;
`;

const RtspUrls = styled.div`
  margin-top: 0.5rem;
  padding-top: 0.5rem;

  border-top: solid 1px ${Colors.GREY_LIGHTEST};
`;