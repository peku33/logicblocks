export const SnapshotDeviceInner: React.VFC<{
  baseUrl: string;
  lastUpdated: Date;
}> = (props) => {
  const { baseUrl, lastUpdated } = props;

  return (
    <a href={`${baseUrl}/full?cache=${lastUpdated.getTime()}`} target="_blank" rel="noreferrer">
      <img src={`${baseUrl}/320?cache=${lastUpdated.getTime()}`} alt="Preview" />
    </a>
  );
};
export const SnapshotDeviceInnerNone: React.VFC = () => {
  return null;
};
