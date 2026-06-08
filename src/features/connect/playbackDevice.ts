import type { ConnectDevicePresence, ConnectSharedPlaybackState } from '~/bindings';

interface ResolveExternalPlaybackDeviceInput {
  connected: boolean;
  ownDeviceId: string | null;
  devices: ConnectDevicePresence[];
  sharedPlayback: ConnectSharedPlaybackState | null;
}

export function resolveCurrentPlaybackDevice({
  connected,
  ownDeviceId,
  devices,
  sharedPlayback,
}: ResolveExternalPlaybackDeviceInput): ConnectDevicePresence | undefined {
  if (!connected || !ownDeviceId || !sharedPlayback?.activeDeviceId) {
    return undefined;
  }

  return devices.find((device) => device.deviceId === sharedPlayback.activeDeviceId);
}

export function resolveExternalPlaybackDevice({
  connected,
  ownDeviceId,
  devices,
  sharedPlayback,
}: ResolveExternalPlaybackDeviceInput): ConnectDevicePresence | undefined {
  if (!connected || !ownDeviceId || !sharedPlayback?.activeDeviceId) {
    return undefined;
  }

  if (sharedPlayback.activeDeviceId === ownDeviceId) {
    return undefined;
  }

  return devices.find((device) => device.deviceId === sharedPlayback.activeDeviceId);
}
