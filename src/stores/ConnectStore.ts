import { createStore } from 'solid-js/store';
import type { ConnectDeviceWithPlayback } from '~/bindings';

export type ConnectServerStatus = 'disabled' | 'checking' | 'available' | 'unavailable';

interface ConnectStore {
  status: ConnectServerStatus;
  message: string | null;
  version: string | null;
  protocolVersion: number | null;
  capabilities: {
    presence: boolean;
    playbackState: boolean;
    handoff: boolean;
    remoteControl: boolean;
  } | null;
  runtime: {
    enabled: boolean;
    connected: boolean;
    message: string | null;
    deviceId: string | null;
    seq: number;
  };
  devices: ConnectDeviceWithPlayback[];
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
});
