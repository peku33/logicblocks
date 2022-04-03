import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import styled from "styled-components";

export const SnapshotDeviceInner: React.VFC<{
  baseUrl: string;
  lastUpdated: Date;
}> = (props) => {
  const { baseUrl, lastUpdated } = props;

  return (
    <SnapshotDeviceInnerUrl href={`${baseUrl}/full?cache=${lastUpdated.getTime()}`} target="_blank" rel="noreferrer">
      <SnapshotDeviceInnerImage src={`${baseUrl}/320?cache=${lastUpdated.getTime()}`} alt="Preview" />
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

export const SnapshotDeviceInnerNone: React.VFC = () => {
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
