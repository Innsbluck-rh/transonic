import type { ConnectDevicePresence, ConnectSharedPlaybackState, PlayingState, SongResponse } from '~/bindings';
import { setConnectStore } from '~/stores/ConnectStore';

export function device(deviceId: string, displayName = deviceId): ConnectDevicePresence {
  return {
    deviceId,
    displayName,
    platform: 'test',
    appVersion: '0.0.0',
    lastSeenAt: '2026-05-25T00:00:00Z',
    online: true,
  };
}

interface SharedPlaybackOptions {
  seq?: number;
  activeDeviceId?: string | null;
  playingState?: PlayingState;
  queue?: SongResponse[];
  currentIndex?: number | null;
  currentPositionMs?: number;
  currentSongId?: string | null;
  updatedAt?: string;
  updateReason?: string | null;
}

// currentSongId follows queue/currentIndex unless given.
export function sharedPlayback(options: SharedPlaybackOptions = {}): ConnectSharedPlaybackState {
  const activeDeviceId = options.activeDeviceId === undefined ? 'remote-device' : options.activeDeviceId;
  const queue = options.queue ?? [];
  const currentIndex = options.currentIndex ?? null;
  return {
    seq: options.seq ?? 1,
    activeDeviceId,
    state: {
      playingState: options.playingState ?? 'playing',
      queue,
      currentIndex,
      playNextQueueLen: 0,
      currentPositionMs: options.currentPositionMs ?? 0,
      currentSongId: options.currentSongId === undefined ? (currentIndex === null ? null : (queue[currentIndex]?.id ?? null)) : options.currentSongId,
    },
    updatedAt: options.updatedAt ?? '2026-05-31T00:00:00Z',
    updatedByDeviceId: activeDeviceId,
    updateReason: options.updateReason === undefined ? 'snapshot' : options.updateReason,
  };
}

export function resetConnectStore() {
  setConnectStore({
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
}

// Connected as 'own-device' and receiving the given shared playback.
export function setSharedPlayback(state: ConnectSharedPlaybackState) {
  setConnectStore('runtime', {
    enabled: true,
    connected: true,
    message: null,
    deviceId: 'own-device',
    seq: 1,
  });
  setConnectStore('sharedPlayback', state);
  setConnectStore('sharedPlaybackReceivedAtMs', 100);
}

// A fully available Connect server, connected as 'local-device'.
export function setConnectAvailable(devices: ConnectDevicePresence[], state: ConnectSharedPlaybackState) {
  setConnectStore({
    status: 'available',
    message: null,
    version: '0.1.0',
    protocolVersion: 2,
    capabilities: {
      presence: true,
      sharedQueue: true,
      sharedPlayback: true,
      playbackTransfer: true,
    },
    runtime: {
      enabled: true,
      connected: true,
      message: null,
      deviceId: 'local-device',
      seq: 1,
    },
    devices,
    sharedPlayback: state,
    sharedPlaybackReceivedAtMs: 0,
  });
}
