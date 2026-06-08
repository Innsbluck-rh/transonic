import { createRoot } from 'solid-js';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ConnectSharedPlaybackState, PlaybackStatus, SongResponse } from '~/bindings';
import { setConnectStore } from '~/stores/ConnectStore';
import { setPlaybackStore } from '~/stores/PlaybackStore';
import { mergePlaybackStatusWithSharedPlayback } from './sharedPlaybackStatus';
import { usePlayback } from './usePlayback';

vi.mock('~/bindings', () => ({
  commands: {},
}));

function song(id: string, title = id): SongResponse {
  return {
    id,
    parentId: null,
    path: null,
    title,
    album: null,
    albumId: null,
    artist: null,
    artistId: null,
    coverArtId: null,
    displayCoverArtId: null,
    track: null,
    discNumber: null,
    year: null,
    duration: null,
    size: null,
    contentType: null,
    suffix: null,
    bitRate: null,
    genre: null,
    created: null,
    starred: null,
    isDirectory: false,
    mediaType: 'song',
  };
}

function playbackStatus(queue: SongResponse[], currentIndex: number | null, overrides: Partial<PlaybackStatus> = {}): PlaybackStatus {
  const status: PlaybackStatus = {
    playingState: queue.length ? 'playing' : 'idle',
    interruptReason: null,
    pendingSeekPositionMs: null,
    gaplessStatus: {
      state: 'unavailable',
      message: 'gapless: unavailable',
    },
    queue,
    currentIndex,
    playNextQueueLen: 0,
    currentPositionMs: 0,
    currentSongId: currentIndex === null ? null : (queue[currentIndex]?.id ?? null),
    playbackError: null,
    activeStreamInfo: null,
    preparedStreamInfo: null,
    lastStreamRequest: null,
    actualStreamInfo: null,
  };

  return { ...status, ...overrides };
}

function sharedPlayback(queue: SongResponse[], currentIndex: number | null): ConnectSharedPlaybackState {
  return {
    seq: 1,
    activeDeviceId: 'remote-device',
    state: {
      playingState: queue.length ? 'playing' : 'idle',
      queue,
      currentIndex,
      playNextQueueLen: 0,
      currentPositionMs: 0,
      currentSongId: currentIndex === null ? null : (queue[currentIndex]?.id ?? null),
    },
    updatedAt: '2026-05-31T00:00:00Z',
    updatedByDeviceId: 'remote-device',
    updateReason: 'snapshot',
  };
}

function resetStores() {
  setPlaybackStore('status', undefined);
  setPlaybackStore('authoritativeReceivedAtMs', null);
  setPlaybackStore('clockMs', 0);
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

describe('usePlayback', () => {
  beforeEach(() => {
    resetStores();
  });

  it('reads the local playback queue when Connect shared playback is unavailable', () => {
    const localQueue = [song('local-1'), song('local-2')];
    setPlaybackStore('status', playbackStatus(localQueue, 1));

    const dispose = createRoot((dispose) => {
      const playback = usePlayback();

      expect(playback.queue().map((entry) => entry.id)).toEqual(['local-1', 'local-2']);
      expect(playback.currentEntry()?.id).toBe('local-2');

      return dispose;
    });

    dispose();
  });

  it('uses the shared playback queue and index while Connect shared playback is active', () => {
    const localQueue = [song('local-1')];
    const remoteQueue = [song('remote-1'), song('remote-2')];
    setPlaybackStore('status', playbackStatus(localQueue, 0));
    setConnectStore('runtime', {
      enabled: true,
      connected: true,
      message: null,
      deviceId: 'own-device',
      seq: 1,
    });
    setConnectStore('sharedPlayback', sharedPlayback(remoteQueue, 1));
    setConnectStore('sharedPlaybackReceivedAtMs', 100);

    const dispose = createRoot((dispose) => {
      const playback = usePlayback();

      expect(playback.queue().map((entry) => entry.id)).toEqual(['remote-1', 'remote-2']);
      expect(playback.currentIndex()).toBe(1);
      expect(playback.currentEntry()?.id).toBe('remote-2');

      return dispose;
    });

    dispose();
  });

  it('keeps local playback details while Connect shared playback provides the shared display state', () => {
    const localQueue = [song('local-1')];
    const remoteQueue = [song('remote-1'), song('remote-2')];
    const localStatus = playbackStatus(localQueue, 0, {
      interruptReason: 'initial_load',
      pendingSeekPositionMs: 5500,
      gaplessStatus: {
        state: 'preparing',
        message: 'gapless: preparing',
      },
      playbackError: {
        message: 'decoder failed',
        handled: false,
      },
    });
    const remoteSharedPlayback = sharedPlayback(remoteQueue, 1);
    setPlaybackStore('status', localStatus);
    setConnectStore('runtime', {
      enabled: true,
      connected: true,
      message: null,
      deviceId: 'own-device',
      seq: 1,
    });
    setConnectStore('sharedPlayback', remoteSharedPlayback);
    setConnectStore('sharedPlaybackReceivedAtMs', 100);

    const mergedStatus = mergePlaybackStatusWithSharedPlayback(localStatus, remoteSharedPlayback);
    expect(mergedStatus.playbackError).toEqual({
      message: 'decoder failed',
      handled: false,
    });

    const dispose = createRoot((dispose) => {
      const playback = usePlayback();

      expect(playback.queue().map((entry) => entry.id)).toEqual(['remote-1', 'remote-2']);
      expect(playback.currentIndex()).toBe(1);
      expect(playback.interruptReason()).toBe('initial_load');
      expect(playback.currentPositionMs()).toBe(5500);
      expect(playback.gaplessStatus()?.state).toBe('preparing');

      return dispose;
    });

    dispose();
  });
});
