import { createStore } from 'solid-js/store';
import type { ConnectDevicePresence, ConnectSharedPlaybackState } from '~/bindings';

export type ConnectServerStatus = 'disabled' | 'checking' | 'available' | 'unavailable';

interface ConnectStore {
  status: ConnectServerStatus;
  message: string | null;
  version: string | null;
  protocolVersion: number | null;
  capabilities: {
    presence: boolean;
    sharedQueue: boolean;
    sharedPlayback: boolean;
    playbackTransfer: boolean;
  } | null;
  runtime: {
    enabled: boolean;
    connected: boolean;
    message: string | null;
    deviceId: string | null;
    seq: number;
  };
  devices: ConnectDevicePresence[];
  sharedPlayback: ConnectSharedPlaybackState | null;
  sharedPlaybackReceivedAtMs: number | null;
}

export const [connectStore, setConnectStore] = createStore<ConnectStore>({
  status: 'disabled',
  message: null,
  version: null,
  protocolVersion: null,
  capabilities: null,
  runtime: {
    enabled: false,
    connected: false,
    message: null,
    deviceId: null,
    seq: 0,
  },
  devices: [],
  sharedPlayback: null,
  sharedPlaybackReceivedAtMs: null,
});
