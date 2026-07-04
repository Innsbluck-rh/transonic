import { beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type ConnectSharedPlaybackState, type ConnectStateSnapshot, type PlayingState, type SongResponse } from '~/bindings';
import { connectStore, setConnectStore } from '~/stores/ConnectStore';
import { refreshConnectStateSnapshot, shouldReuseSharedPlaybackReceivedAtMs } from './service';

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

function song(id: string): SongResponse {
  return { id } as SongResponse;
}

function sharedPlayback(
  overrides: {
    seq?: number;
    activeDeviceId?: string | null;
    playingState?: PlayingState;
    currentIndex?: number | null;
    currentPositionMs?: number;
    currentSongId?: string | null;
    queueIds?: string[];
    updatedAt?: string;
    updateReason?: string | null;
  } = {}
): ConnectSharedPlaybackState {
  return {
    seq: overrides.seq ?? 1,
    activeDeviceId: overrides.activeDeviceId === undefined ? 'remote-device' : overrides.activeDeviceId,
    state: {
      playingState: overrides.playingState ?? 'playing',
      queue: (overrides.queueIds ?? []).map(song),
      currentIndex: overrides.currentIndex === undefined ? 0 : overrides.currentIndex,
      playNextQueueLen: 0,
      currentPositionMs: overrides.currentPositionMs ?? 0,
      currentSongId: overrides.currentSongId === undefined ? 'song-a' : overrides.currentSongId,
    },
    updatedAt: overrides.updatedAt ?? '2026-05-31T00:00:00Z',
    updatedByDeviceId: 'remote-device',
    updateReason: overrides.updateReason === undefined ? 'report' : overrides.updateReason,
  };
}

function snapshot(sharedPlayback: ConnectSharedPlaybackState | null): ConnectStateSnapshot {
  return {
    runtime: {
      enabled: true,
      connected: true,
      message: null,
      serverUrl: 'https://connect.example.test',
      deviceId: 'own-device',
      seq: sharedPlayback?.seq ?? 0,
    },
    devices: [],
    sharedPlayback,
  };
}

function resetConnectStore() {
  setConnectStore({
    status: 'available',
    message: null,
    version: null,
    protocolVersion: null,
    capabilities: null,
    runtime: {
      enabled: true,
      connected: true,
      message: null,
      deviceId: 'own-device',
      seq: 0,
    },
    devices: [],
    sharedPlayback: null,
    sharedPlaybackReceivedAtMs: null,
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetConnectStore();
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
    const previous = sharedPlayback({ currentIndex: 0, currentPositionMs: 0, currentSongId: null, queueIds: ['song-a'] });
    const next = sharedPlayback({ currentIndex: 0, currentPositionMs: 0, currentSongId: null, queueIds: ['song-b'] });

    expect(shouldReuseSharedPlaybackReceivedAtMs(previous, next, 100)).toBe(false);
  });
});

describe('refreshConnectStateSnapshot', () => {
  it('refreshes the stored receive time when a same-seq snapshot moves to the next track', async () => {
    const previous = sharedPlayback({ currentIndex: 0, currentPositionMs: 18_000, currentSongId: 'song-a', queueIds: ['song-a', 'song-b'] });
    const next = sharedPlayback({
      currentIndex: 1,
      currentPositionMs: 0,
      currentSongId: 'song-b',
      queueIds: ['song-a', 'song-b'],
      updatedAt: '2026-05-31T00:00:18Z',
    });
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
