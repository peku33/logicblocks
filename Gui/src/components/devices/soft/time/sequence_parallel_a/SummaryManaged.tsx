import { deviceClassPostEmpty, deviceClassPostJsonEmpty } from "@/components/devices/Device";
import { DeviceSummaryManaged } from "@/components/devices/DeviceSummaryManaged";
import { useDeviceSummary } from "@/components/devices/DeviceSummaryService";
import { useCallback } from "react";
import Component, { Data } from "./Summary";

const ManagedComponent: DeviceSummaryManaged = (props) => {
  const { deviceId } = props;

  const data = useDeviceSummary<Data>(deviceId);

  const onDeviceDisable = useCallback(async () => {
    await deviceClassPostEmpty(deviceId, "/device/disable");
  }, [deviceId]);
  const onDevicePause = useCallback(async () => {
    await deviceClassPostEmpty(deviceId, "/device/pause");
  }, [deviceId]);
  const onDeviceEnable = useCallback(async () => {
    await deviceClassPostEmpty(deviceId, "/device/enable");
  }, [deviceId]);

  const onChannelsAllClear = useCallback(async () => {
    await deviceClassPostEmpty(deviceId, "/channels/all/clear");
  }, [deviceId]);
  const onChannelsAllAdd = useCallback(
    async (multiplier: number) => {
      await deviceClassPostJsonEmpty(deviceId, "/channels/all/add", multiplier);
    },
    [deviceId],
  );

  const onChannelDisable = useCallback(
    async (channelId: number) => {
      await deviceClassPostEmpty(deviceId, `/channels/${channelId}/disable`);
    },
    [deviceId],
  );
  const onChannelPause = useCallback(
    async (channelId: number) => {
      await deviceClassPostEmpty(deviceId, `/channels/${channelId}/pause`);
    },
    [deviceId],
  );
  const onChannelEnable = useCallback(
    async (channelId: number) => {
      await deviceClassPostEmpty(deviceId, `/channels/${channelId}/enable`);
    },
    [deviceId],
  );
  const onChannelClear = useCallback(
    async (channelId: number) => {
      await deviceClassPostEmpty(deviceId, `/channels/${channelId}/clear`);
    },
    [deviceId],
  );
  const onChannelAdd = useCallback(
    async (channelId: number, multiplier: number) => {
      await deviceClassPostJsonEmpty(deviceId, `/channels/${channelId}/add`, multiplier);
    },
    [deviceId],
  );
  const onChannelMoveFront = useCallback(
    async (channelId: number) => {
      await deviceClassPostEmpty(deviceId, `/channels/${channelId}/move-front`);
    },
    [deviceId],
  );
  const onChannelMoveBack = useCallback(
    async (channelId: number) => {
      await deviceClassPostEmpty(deviceId, `/channels/${channelId}/move-back`);
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
