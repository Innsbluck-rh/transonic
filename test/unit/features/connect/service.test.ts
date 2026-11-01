import { beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type ConnectSharedPlaybackState, type ConnectStateSnapshot } from '~/bindings';
import { refreshConnectStateSnapshot, shouldReuseSharedPlaybackReceivedAtMs } from '~/features/connect/service';
import { connectStore, setConnectStore } from '~/stores/ConnectStore';
import { resetConnectStore, sharedPlayback } from '~test/fixtures/connect';
import { song } from '~test/fixtures/playback';

const bindingMocks = vi.hoisted(() => ({
  connectGetStateSnapshot: vi.fn(),
  connectStateUpdatedListen: vi.fn(),
}));

vi.mock('~/bindings', () => ({
  commands: {
    connectGetStateSnapshot: bindingMocks.connectGetStateSnapshot,
  },
  events: {
    connectStateUpdated: {
      listen: bindingMocks.connectStateUpdatedListen,
    },
  },
}));

function snapshot(sharedPlayback: ConnectSharedPlaybackState | null): ConnectStateSnapshot {
  return {
    runtime: {
      enabled: true,
      connected: true,
      message: null,
      serverUrl: 'https://connect.example.test',
      deviceId: 'own-device',
      seq: sharedPlayback?.seq ?? 0,
      queueResolution: null,
    },
    devices: [],
    sharedPlayback,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  resetConnectStore();
  setConnectStore('status', 'available');
  setConnectStore('runtime', {
    enabled: true,
    connected: true,
    message: null,
    deviceId: 'own-device',
    seq: 0,
  });
});

describe('shouldReuseSharedPlaybackReceivedAtMs', () => {
  it('reuses the previous receive time for the same seq and timeline base', () => {
    const previous = sharedPlayback({ updateReason: 'snapshot' });
    const next = sharedPlayback({ updateReason: 'report' });

    expect(shouldReuseSharedPlaybackReceivedAtMs(previous, next, 100)).toBe(true);
  });

  it('does not reuse the previous receive time when there is no previous receive time', () => {
    const previous = sharedPlayback();
    const next = sharedPlayback();

    expect(shouldReuseSharedPlaybackReceivedAtMs(previous, next, null)).toBe(false);
  });

  it('resets the receive time when seq changes', () => {
    const previous = sharedPlayback({ seq: 1 });
    const next = sharedPlayback({ seq: 2 });

    expect(shouldReuseSharedPlaybackReceivedAtMs(previous, next, 100)).toBe(false);
  });

  it('resets the receive time when updatedAt changes for the same seq', () => {
    const previous = sharedPlayback({ updatedAt: '2026-05-31T00:00:00Z' });
    const next = sharedPlayback({ updatedAt: '2026-05-31T00:00:01Z' });

    expect(shouldReuseSharedPlaybackReceivedAtMs(previous, next, 100)).toBe(false);
  });

  it('resets the receive time when currentPositionMs changes for the same seq', () => {
    const previous = sharedPlayback({ currentPositionMs: 0 });
    const next = sharedPlayback({ currentPositionMs: 500 });

    expect(shouldReuseSharedPlaybackReceivedAtMs(previous, next, 100)).toBe(false);
  });

  it('resets the receive time when the same seq moves to the next track at position zero', () => {
    const previous = sharedPlayback({ currentIndex: 0, currentPositionMs: 0, currentSongId: 'song-a' });
    const next = sharedPlayback({ currentIndex: 1, currentPositionMs: 0, currentSongId: 'song-b' });

    expect(shouldReuseSharedPlaybackReceivedAtMs(previous, next, 100)).toBe(false);
  });

  it('resets the receive time when the queued current entry changes without a currentSongId', () => {
    const previous = sharedPlayback({ currentIndex: 0, currentPositionMs: 0, currentSongId: null, queue: [song('song-a')] });
    const next = sharedPlayback({ currentIndex: 0, currentPositionMs: 0, currentSongId: null, queue: [song('song-b')] });

    expect(shouldReuseSharedPlaybackReceivedAtMs(previous, next, 100)).toBe(false);
  });
});

describe('refreshConnectStateSnapshot', () => {
  it('refreshes the stored receive time when a same-seq snapshot moves to the next track', async () => {
    const queue = [song('song-a'), song('song-b')];
    const previous = sharedPlayback({ queue, currentIndex: 0, currentPositionMs: 18_000 });
    const next = sharedPlayback({ queue, currentIndex: 1, currentPositionMs: 0, updatedAt: '2026-05-31T00:00:18Z' });
    const nowSpy = vi.spyOn(performance, 'now').mockReturnValue(18_100);
    setConnectStore('sharedPlayback', previous);
    setConnectStore('sharedPlaybackReceivedAtMs', 100);
    vi.mocked(commands.connectGetStateSnapshot).mockResolvedValue(snapshot(next));

    try {
      await refreshConnectStateSnapshot();
    } finally {
      nowSpy.mockRestore();
    }

    expect(connectStore.sharedPlayback).toEqual(next);
    expect(connectStore.sharedPlaybackReceivedAtMs).toBe(18_100);
  });
});
