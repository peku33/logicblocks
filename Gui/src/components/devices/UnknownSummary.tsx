import React from "react";
import styled from "styled-components";

const UnknownDevice: React.FC<{
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

export default UnknownDevice;

const DetailsSpan = styled.span`
  word-break: break-all;
`;
