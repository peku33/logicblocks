import { ComponentManagedBase } from "components/devices/SummaryManaged";
import { useCallback } from "react";
import { devicePostEmpty, devicePostJsonEmpty, useDeviceSummaryData } from "services/LogicDevicesRunner";
import Component, { Data } from "./Summary";

const ComponentManaged: ComponentManagedBase = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummaryData<Data>(deviceId);

  const onDeviceDisable = useCallback((): void => {
    devicePostEmpty(deviceId, "/device/disable");
  }, [deviceId]);
  const onDevicePause = useCallback((): void => {
    devicePostEmpty(deviceId, "/device/pause");
  }, [deviceId]);
  const onDeviceEnable = useCallback((): void => {
    devicePostEmpty(deviceId, "/device/enable");
  }, [deviceId]);

  const onChannelsAllClear = useCallback((): void => {
    devicePostEmpty(deviceId, "/channels/all/clear");
  }, [deviceId]);
  const onChannelsAllAdd = useCallback(
    (multiplier: number): void => {
      devicePostJsonEmpty(deviceId, "/channels/all/add", multiplier);
    },
    [deviceId],
  );

  const onChannelDisable = useCallback(
    (channelId: number) => {
      devicePostEmpty(deviceId, `/channels/${channelId}/disable`);
    },
    [deviceId],
  );
  const onChannelPause = useCallback(
    (channelId: number) => {
      devicePostEmpty(deviceId, `/channels/${channelId}/pause`);
    },
    [deviceId],
  );
  const onChannelEnable = useCallback(
    (channelId: number) => {
      devicePostEmpty(deviceId, `/channels/${channelId}/enable`);
    },
    [deviceId],
  );
  const onChannelClear = useCallback(
    (channelId: number) => {
      devicePostEmpty(deviceId, `/channels/${channelId}/clear`);
    },
    [deviceId],
  );
  const onChannelAdd = useCallback(
    (channelId: number, multiplier: number) => {
      devicePostJsonEmpty(deviceId, `/channels/${channelId}/add`, multiplier);
    },
    [deviceId],
  );
  const onChannelMoveFront = useCallback(
    (channelId: number) => {
      devicePostEmpty(deviceId, `/channels/${channelId}/move-front`);
    },
    [deviceId],
  );
  const onChannelMoveBack = useCallback(
    (channelId: number) => {
      devicePostEmpty(deviceId, `/channels/${channelId}/move-back`);
    },
    [deviceId],
  );

  return (
    <Component
      data={data}
      onDeviceDisable={onDeviceDisable}
      onDevicePause={onDevicePause}
      onDeviceEnable={onDeviceEnable}
      onChannelsAllClear={onChannelsAllClear}
      onChannelsAllAdd={onChannelsAllAdd}
      onChannelDisable={onChannelDisable}
      onChannelPause={onChannelPause}
      onChannelEnable={onChannelEnable}
      onChannelClear={onChannelClear}
      onChannelAdd={onChannelAdd}
      onChannelMoveFront={onChannelMoveFront}
      onChannelMoveBack={onChannelMoveBack}
    />
  );
};
export default ComponentManaged;
