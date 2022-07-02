import Colors from "components/common/Colors";
import { TopicPath, useVersions } from "lib/SSETopic";
import { useMemo } from "react";
import styled from "styled-components";
import { DeviceId, endpointBuild } from "./Device";
import { DeviceSummaryContext } from "./DeviceSummary";
import DeviceSummaryManagedWrapper from "./DeviceSummaryManagedWrapper";

const DeviceSummaryManagedWrapperList: React.FC<{
  deviceIds: DeviceId[];
}> = (props) => {
  const { deviceIds } = props;

  const versions = useVersions(endpointBuild("/devices/gui-summary-sse"), deviceIds.map(buildTopicPathFromDeviceId));

  return (
    <Grid>
      {deviceIds.map((deviceId, index) => (
        <GridItem key={deviceId}>
          <DeviceSummaryManagedWrapperListItem deviceId={deviceId} version={versions[index]} />
        </GridItem>
      ))}
    </Grid>
  );
};
export default DeviceSummaryManagedWrapperList;

const DeviceSummaryManagedWrapperListItem: React.FC<{
  deviceId: DeviceId;
  version: number;
}> = (props) => {
  const { deviceId, version } = props;

  const deviceSummaryContext = useMemo(() => ({ deviceId, version } as DeviceSummaryContext), [deviceId, version]);

  return <DeviceSummaryManagedWrapper deviceSummaryContext={deviceSummaryContext} />;
};

function buildTopicPathFromDeviceId(deviceId: DeviceId): TopicPath {
  return [deviceId];
}

const Grid = styled.div`
  margin: 0.25rem;

  display: grid;
  grid-gap: 0.25rem;

  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  grid-auto-rows: auto;

  align-items: start;
  justify-content: center;
`;
const GridItem = styled.div`
  padding: 0.25rem;

  border: solid 1px ${Colors.GREY};
`;
