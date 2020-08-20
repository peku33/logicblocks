import React from "react";

const UnknownDevice: React.FC<{
  deviceId: number;
  deviceClass: string;
}> = (props) => {
  const { deviceId, deviceClass } = props;
  return (
    <>
      Unknown device #{deviceId} ({deviceClass})
    </>
  );
};

export default UnknownDevice;
