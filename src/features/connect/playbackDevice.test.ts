import { describe, expect, it } from 'vitest';
import type { ConnectDevicePresence, ConnectSharedPlaybackState } from '~/bindings';
import { resolveExternalPlaybackDevice } from './playbackDevice';

function device(deviceId: string, displayName = deviceId): ConnectDevicePresence {
  return {
    deviceId,
    displayName,
    platform: 'test',
    appVersion: '0.0.0',
    lastSeenAt: '2026-05-25T00:00:00Z',
    online: true,
  };
}

function sharedPlayback(activeDeviceId: string | null): ConnectSharedPlaybackState {
  return {
    seq: 1,
    activeDeviceId,
    state: {
      playingState: 'playing',
      queue: [],
      currentIndex: null,
      currentPositionMs: 0,
      currentSongId: null,
      playNextQueueLen: 0,
    },
    updatedAt: '2026-05-25T00:00:00Z',
    updatedByDeviceId: activeDeviceId,
    updateReason: 'snapshot',
  };
}

describe('resolveExternalPlaybackDevice', () => {
  it('does not resolve a playback device before the local runtime identity is known', () => {
    const result = resolveExternalPlaybackDevice({
      connected: false,
      ownDeviceId: null,
      devices: [device('own-device', 'This device')],
      sharedPlayback: sharedPlayback('own-device'),
    });

    expect(result).toBeUndefined();
  });

  it('does not resolve the local device as external playback', () => {
    const result = resolveExternalPlaybackDevice({
      connected: true,
      ownDeviceId: 'own-device',
      devices: [device('own-device', 'This device')],
      sharedPlayback: sharedPlayback('own-device'),
    });

    expect(result).toBeUndefined();
  });

  it('resolves the active remote device once the local identity is known', () => {
    const remoteDevice = device('remote-device', 'Living Room');
    const result = resolveExternalPlaybackDevice({
      connected: true,
      ownDeviceId: 'own-device',
      devices: [device('own-device', 'This device'), remoteDevice],
      sharedPlayback: sharedPlayback('remote-device'),
    });

    expect(result).toBe(remoteDevice);
  });

  it('ignores an active device that is not in the presence list', () => {
    const result = resolveExternalPlaybackDevice({
      connected: true,
      ownDeviceId: 'own-device',
      devices: [device('own-device', 'This device')],
      sharedPlayback: sharedPlayback('missing-device'),
    });

    expect(result).toBeUndefined();
  });
});
