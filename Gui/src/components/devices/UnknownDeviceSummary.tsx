import styled from "styled-components";

const Summary: React.VFC<{
  deviceId: number;
  deviceClass: string;
}> = (props) => {
  const { deviceId, deviceClass } = props;

  return (
    <Wrapper>
      Unknown device #<DetailsSpan>{deviceId}</DetailsSpan> <DetailsSpan>({deviceClass})</DetailsSpan>
    </Wrapper>
  );
};
export default Summary;

const Wrapper = styled.div`
  font-size: x-small;
`;

const DetailsSpan = styled.span`
  word-break: break-all;
`;
