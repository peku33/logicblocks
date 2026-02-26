import { Chip, ChipsGroup, ChipType } from "@/components/common/Chips";
import GaugeLinearRatio from "@/components/datatypes/ratio/GaugeLinear";
import { clamp } from "@/util/Number";
import { useMemo } from "react";
import styled from "styled-components";
import { useCounter, useInterval } from "usehooks-ts";

export type Direction = "Up" | "Down";

// Data
export interface Data {
  state: DataState;
}

export type DataState = DataStateUncalibrated | DataStateCalibrating | DataStateStopped | DataStateMoving;

export interface DataStateUncalibrated {
  state: "Uncalibrated";
}
export function dataStateIsUncalibrated(dataState: DataState): dataState is DataStateUncalibrated {
  return dataState.state === "Uncalibrated";
}

export interface DataStateCalibrating {
  state: "Calibrating";
  started_ago_seconds: number;
  direction: Direction;
  duration_seconds: number;
}
export function dataStateIsCalibrating(dataState: DataState): dataState is DataStateCalibrating {
  return dataState.state === "Calibrating";
}

export interface DataStateStopped {
  state: "Stopped";
  position: number;
  uncertainty_relative: number;
}
export function dataStateIsStopped(dataState: DataState): dataState is DataStateStopped {
  return dataState.state === "Stopped";
}

export interface DataStateMoving {
  state: "Moving";
  started_ago_seconds: number;
  started_position: number;
  setpoint: number;
  duration_seconds: number;
  direction: Direction;
}
export function dataStateIsMoving(dataState: DataState): dataState is DataStateMoving {
  return dataState.state === "Moving";
}

// Animation
const ANIMATION_INTERVAL_MS = 1000;

type Animation = AnimationCalibrating | AnimationMoving;

interface AnimationCalibrating {
  state: "Calibrating";
  progress: number;
}
function animationIsCalibrating(animation: Animation): animation is AnimationCalibrating {
  return animation.state === "Calibrating";
}
function animationCalibratingBuild(
  dataStateCalibrating: DataStateCalibrating,
  elapsed_seconds: number,
): AnimationCalibrating {
  const progress = clamp(
    (dataStateCalibrating.started_ago_seconds + elapsed_seconds) / dataStateCalibrating.duration_seconds,
    0.0,
    1.0,
  );

  return {
    state: "Calibrating",
    progress,
  };
}

interface AnimationMoving {
  state: "Moving";
  position: number;
}
function animationIsMoving(animation: Animation): animation is AnimationMoving {
  return animation.state === "Moving";
}
function animationMovingBuild(dataStateMoving: DataStateMoving, elapsed_seconds: number): AnimationMoving {
  const progress = clamp(
    (dataStateMoving.started_ago_seconds + elapsed_seconds) / dataStateMoving.duration_seconds,
    0.0,
    1.0,
  );

  const position =
    dataStateMoving.started_position + progress * (dataStateMoving.setpoint - dataStateMoving.started_position);

  return {
    state: "Moving",
    position,
  };
}

function animationBuild(data: Data | undefined, elapsedSeconds: number): Animation | undefined {
  if (data === undefined) {
    return undefined;
  }

  if (dataStateIsCalibrating(data.state)) {
    return animationCalibratingBuild(data.state, elapsedSeconds);
  }

  if (dataStateIsMoving(data.state)) {
    return animationMovingBuild(data.state, elapsedSeconds);
  }

  return undefined;
}

const Component: React.FC<{
  data: Data | undefined;
}> = (props) => {
  const { data } = props;

  // timestamp when data was loaded
  const dataLoadTimestamp = useMemo(() => {
    return Date.now();
  }, [data]);

  // counter that causes animation memo to update
  const animationCounter = useCounter(0);

  // memoized animation details, updated on every data change and animation counter tick
  const animation: Animation | undefined = useMemo(() => {
    const elapsedSeconds = (Date.now() - dataLoadTimestamp) / 1000;

    const animation = animationBuild(data, elapsedSeconds);

    return animation;
  }, [data, dataLoadTimestamp, animationCounter.count]);

  // interval to bump the counter which will cause recalculating of animation
  useInterval(
    () => {
      animationCounter.increment();
    },
    animation !== undefined ? ANIMATION_INTERVAL_MS : null,
  );

  return (
    <Wrapper>
      <Section>
        <SectionTitle>Controller Status</SectionTitle>
        <SectionContent>
          <ChipsGroup>
            <Chip type={ChipType.WARNING} enabled={data != undefined && dataStateIsUncalibrated(data.state)}>
              Uncalibrated
            </Chip>
            <Chip type={ChipType.WARNING} enabled={data != undefined && dataStateIsCalibrating(data.state)}>
              Calibrating
            </Chip>
            <Chip type={ChipType.OK} enabled={data != undefined && dataStateIsStopped(data.state)}>
              Stopped
            </Chip>
            <Chip type={ChipType.INFO} enabled={data != undefined && dataStateIsMoving(data.state)}>
              Moving
            </Chip>
          </ChipsGroup>
        </SectionContent>
        {data !== undefined && dataStateIsCalibrating(data.state) ? (
          <Section>
            <SectionTitle>Calibrating</SectionTitle>
            <SectionContent>
              <Chip type={ChipType.INFO} enabled={data.state.direction === "Up"}>
                Up
              </Chip>
              <Chip type={ChipType.INFO} enabled={data.state.direction === "Down"}>
                Down
              </Chip>
            </SectionContent>
            {animation !== undefined && animationIsCalibrating(animation) ? (
              <SectionContent>
                <GaugeLinearRatio value={animation.progress}>Progress</GaugeLinearRatio>
              </SectionContent>
            ) : (
              <SectionContent>Estimated total time: {data.state.duration_seconds.toFixed(1)}s</SectionContent>
            )}
          </Section>
        ) : null}
        {data !== undefined && dataStateIsStopped(data.state) ? (
          <Section>
            <SectionTitle>Stopped</SectionTitle>
            <SectionContent>
              <GaugeLinearRatio value={data.state.position}>Position</GaugeLinearRatio>
              <GaugeLinearRatio value={data.state.uncertainty_relative}>Uncertainty</GaugeLinearRatio>
            </SectionContent>
          </Section>
        ) : null}
        {data !== undefined && dataStateIsMoving(data.state) ? (
          <Section>
            <SectionTitle>Moving</SectionTitle>
            <SectionContent>
              <Chip type={ChipType.INFO} enabled={data.state.direction === "Up"}>
                Up
              </Chip>
              <Chip type={ChipType.INFO} enabled={data.state.direction === "Down"}>
                Down
              </Chip>
            </SectionContent>
            <SectionContent>
              <GaugeLinearRatio value={data.state.setpoint}>Setpoint</GaugeLinearRatio>
            </SectionContent>
            {animation !== undefined && animationIsMoving(animation) ? (
              <SectionContent>
                <GaugeLinearRatio value={animation.position}>Position</GaugeLinearRatio>
              </SectionContent>
            ) : (
              <SectionContent>Estimated total time: {data.state.duration_seconds.toFixed(1)}s</SectionContent>
            )}
          </Section>
        ) : null}
      </Section>
    </Wrapper>
  );
};
export default Component;

const Wrapper = styled.div``;

const Section = styled.div`
  margin-bottom: 0.5rem;
`;
const SectionTitle = styled.div`
  font-weight: bold;
`;
const SectionContent = styled.div`
  padding-left: 1rem;
  & > * {
    margin-bottom: 0.25rem;
  }
`;
