import styled from "styled-components";

const Summary: React.VFC<{
  deviceId: number;
  deviceClass: string;
}> = (props) => {
  const { deviceId, deviceClass } = props;
  return (
    <>
      Unknown device #<DetailsSpan>{deviceId}</DetailsSpan>
      <DetailsSpan>({deviceClass})</DetailsSpan>
    </>
  );
};
export default Summary;

const DetailsSpan = styled.span`
  word-break: break-all;
`;
