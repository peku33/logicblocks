import { deviceClassPostEmpty, deviceClassPostJsonEmpty } from "components/devices/Device";
import { useDeviceSummary } from "components/devices/DeviceSummary";
import { DeviceSummaryManaged } from "components/devices/DeviceSummaryManaged";
import { useCallback } from "react";
import Component, { Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceSummaryContext } = props;
  const { deviceId } = deviceSummaryContext;

  const data = useDeviceSummary<Data>(deviceSummaryContext);

  const onDeviceDisable = useCallback((): void => {
    deviceClassPostEmpty(deviceId, "/device/disable");
  }, [deviceId]);
  const onDevicePause = useCallback((): void => {
    deviceClassPostEmpty(deviceId, "/device/pause");
  }, [deviceId]);
  const onDeviceEnable = useCallback((): void => {
    deviceClassPostEmpty(deviceId, "/device/enable");
  }, [deviceId]);

  const onChannelsAllClear = useCallback((): void => {
    deviceClassPostEmpty(deviceId, "/channels/all/clear");
  }, [deviceId]);
  const onChannelsAllAdd = useCallback(
    (multiplier: number): void => {
      deviceClassPostJsonEmpty(deviceId, "/channels/all/add", multiplier);
    },
    [deviceId],
  );

  const onChannelDisable = useCallback(
    (channelId: number) => {
      deviceClassPostEmpty(deviceId, `/channels/${channelId}/disable`);
    },
    [deviceId],
  );
  const onChannelPause = useCallback(
    (channelId: number) => {
      deviceClassPostEmpty(deviceId, `/channels/${channelId}/pause`);
    },
    [deviceId],
  );
  const onChannelEnable = useCallback(
    (channelId: number) => {
      deviceClassPostEmpty(deviceId, `/channels/${channelId}/enable`);
    },
    [deviceId],
  );
  const onChannelClear = useCallback(
    (channelId: number) => {
      deviceClassPostEmpty(deviceId, `/channels/${channelId}/clear`);
    },
    [deviceId],
  );
  const onChannelAdd = useCallback(
    (channelId: number, multiplier: number) => {
      deviceClassPostJsonEmpty(deviceId, `/channels/${channelId}/add`, multiplier);
    },
    [deviceId],
  );
  const onChannelMoveFront = useCallback(
    (channelId: number) => {
      deviceClassPostEmpty(deviceId, `/channels/${channelId}/move-front`);
    },
    [deviceId],
  );
  const onChannelMoveBack = useCallback(
    (channelId: number) => {
      deviceClassPostEmpty(deviceId, `/channels/${channelId}/move-back`);
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
export default ManagedComponent;
