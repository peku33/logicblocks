import { Line } from "@/components/common/Line";
import styled from "styled-components";
import HardwareRunnerSummary, { Data as HardwareRunnerData } from "./hardware_runner/Summary";

export interface Data<D extends object> {
  hardware_runner: HardwareRunnerData;
  device: D;
}

export function makeAvrV1Summary<D extends object>(
  SummaryInnerComponent: React.ComponentType<{ data: D | undefined }>,
): React.FC<{
  data: Data<D> | undefined;
}> {
  const Summary: React.FC<{
    data: Data<D> | undefined;
  }> = (props) => {
    const { data } = props;

    return (
      <Wrapper>
        <HardwareRunnerWrapper>
          <HardwareRunnerSummary data={data?.hardware_runner} />
        </HardwareRunnerWrapper>
        <Line />
        <DeviceComponentWrapper>
          <SummaryInnerComponent data={data?.device} />
        </DeviceComponentWrapper>
      </Wrapper>
    );
  };

  return Summary;
}

const Wrapper = styled.div`
  display: flex;
  flex-direction: column;
`;
const HardwareRunnerWrapper = styled.div``;
const DeviceComponentWrapper = styled.div``;
