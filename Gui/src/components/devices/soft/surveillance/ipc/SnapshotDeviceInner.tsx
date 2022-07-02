import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { urlBuild } from "lib/Api";
import { useMemo } from "react";
import styled from "styled-components";

export const SnapshotDeviceInner: React.FC<{
  endpoint: string;
  lastUpdated: Date;
}> = (props) => {
  const { endpoint, lastUpdated } = props;

  const urlFull = useMemo(() => urlBuild(`${endpoint}/full?cache=${lastUpdated.getTime()}`), [endpoint, lastUpdated]);
  const url320 = useMemo(() => urlBuild(`${endpoint}/320?cache=${lastUpdated.getTime()}`), [endpoint, lastUpdated]);

  return (
    <SnapshotDeviceInnerUrl href={urlFull} target="_blank" rel="noreferrer">
      <SnapshotDeviceInnerImage src={url320} alt="Preview" />
    </SnapshotDeviceInnerUrl>
  );
};
const SnapshotDeviceInnerUrl = styled.a``;
const SnapshotDeviceInnerImage = styled.img`
  display: block;
  margin: auto;

  max-width: 100%;
  max-height: 100%;
`;

export const SnapshotDeviceInnerNone: React.FC<{}> = () => {
  return (
    <SnapshotDeviceInnerNoneInner>
      <FontAwesomeIcon icon={["fas", "circle-notch"]} spin />
    </SnapshotDeviceInnerNoneInner>
  );
};
const SnapshotDeviceInnerNoneInner = styled.div`
  padding: 1rem;
  font-size: 4rem;
`;
