import React, { useEffect, useState } from "react";
import { Card, Loader } from "semantic-ui-react";
import { DeviceListManager } from "../../services/DevicePool";
import { DeviceComponent, getComponentClassForDevice } from "./DeviceComponentFactory";
import DeviceContext from "./DeviceContext";

interface DeviceListItem {
  deviceComponent: DeviceComponent;
  deviceContext: DeviceContext;
}

const DevicePool: React.FC = () => {
  const [deviceListItems, setDeviceListItems] = useState<DeviceListItem[]>([]);

  useEffect(() => DeviceListManager.getInstance().reactHook((newDeviceListItems) => {
    setDeviceListItems(newDeviceListItems.map((newDeviceListItem) => ({
      deviceComponent: getComponentClassForDevice(newDeviceListItem.device_class),
      deviceContext: new DeviceContext(newDeviceListItem.device_id),
    })));
  }), []);

  if (!deviceListItems) { return (<Loader active />); }

  return (
    <div style={{ padding: 10 }}>
      <Card.Group
        stackable
      >
        {deviceListItems.map((deviceListItem) => {
          const DeviceComponentType = deviceListItem.deviceComponent;
          return (
            <Card key={deviceListItem.deviceContext.deviceId}>
              <Card.Content>
                <DeviceComponentType
                  deviceContext={deviceListItem.deviceContext}
                />
              </Card.Content>
            </Card>
          );
        })}
      </Card.Group>
    </div>
  );
};
export default DevicePool;
