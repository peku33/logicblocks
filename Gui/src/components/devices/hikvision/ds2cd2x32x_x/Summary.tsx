import { ButtonGroup, ButtonLink } from "components/common/Button";
import { Chip, ChipsGroup, ChipType } from "components/common/Chips";
import { Line } from "components/common/Line";
import {
  SnapshotDeviceInner,
  SnapshotDeviceInnerNone,
} from "components/devices/soft/surveillance/ipc/SnapshotDeviceInner";
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
  sub: string;
}
export interface DataEvents {
  camera_failure: boolean;
  video_loss: boolean;
  tampering_detection: boolean;
  motion_detection: boolean;
  line_detection: boolean;
  field_detection: boolean;
}

const Component: React.FC<{
  data: Data | undefined;
  snapshotEndpoint: string | undefined;
}> = (props) => {
  const { data, snapshotEndpoint } = props;

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
              <Chip type={ChipType.ERROR} enabled={data.events.camera_failure}>
                Camera failure
              </Chip>
              <Chip type={ChipType.ERROR} enabled={data.events.video_loss}>
                Video Loss
              </Chip>
              <Chip type={ChipType.WARNING} enabled={data.events.tampering_detection}>
                Tampering detection
              </Chip>
              <Chip type={ChipType.INFO} enabled={data.events.motion_detection}>
                Motion detection
              </Chip>
              <Chip type={ChipType.INFO} enabled={data.events.line_detection}>
                Line detection
              </Chip>
              <Chip type={ChipType.INFO} enabled={data.events.field_detection}>
                Field detection
              </Chip>
            </ChipsGroup>
          ) : null}
        </Events>
      </Header>
      <Line />
      <Snapshot>
        {snapshotEndpoint !== undefined &&
        data !== undefined &&
        dataIsRunning(data) &&
        data.snapshot_updated !== null ? (
          <SnapshotDeviceInner endpoint={snapshotEndpoint} lastUpdated={new Date(data.snapshot_updated)} />
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
            <ButtonLink targetBlank href={data.rtsp_urls.sub}>
              Sub Stream
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
