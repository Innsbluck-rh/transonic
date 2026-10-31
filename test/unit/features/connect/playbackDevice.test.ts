import { describe, expect, it } from 'vitest';
import { resolveExternalPlaybackDevice } from '~/features/connect/playbackDevice';
import { device, sharedPlayback } from '~test/fixtures/connect';

describe('resolveExternalPlaybackDevice', () => {
  it('does not resolve a playback device before the local runtime identity is known', () => {
    const result = resolveExternalPlaybackDevice({
      connected: false,
      ownDeviceId: null,
      devices: [device('own-device', 'This device')],
      sharedPlayback: sharedPlayback({ activeDeviceId: 'own-device' }),
    });

    expect(result).toBeUndefined();
  });

  it('does not resolve the local device as external playback', () => {
    const result = resolveExternalPlaybackDevice({
      connected: true,
      ownDeviceId: 'own-device',
      devices: [device('own-device', 'This device')],
      sharedPlayback: sharedPlayback({ activeDeviceId: 'own-device' }),
    });

    expect(result).toBeUndefined();
  });

  it('resolves the active remote device once the local identity is known', () => {
    const remoteDevice = device('remote-device', 'Living Room');
    const result = resolveExternalPlaybackDevice({
      connected: true,
      ownDeviceId: 'own-device',
      devices: [device('own-device', 'This device'), remoteDevice],
      sharedPlayback: sharedPlayback({ activeDeviceId: 'remote-device' }),
    });

    expect(result).toBe(remoteDevice);
  });

  it('ignores an active device that is not in the presence list', () => {
    const result = resolveExternalPlaybackDevice({
      connected: true,
      ownDeviceId: 'own-device',
      devices: [device('own-device', 'This device')],
      sharedPlayback: sharedPlayback({ activeDeviceId: 'missing-device' }),
    });

    expect(result).toBeUndefined();
  });
});
