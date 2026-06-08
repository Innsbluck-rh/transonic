// @vitest-environment jsdom

import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ConnectSharedPlaybackState, PlaybackStatus, SongResponse } from '~/bindings';
import { setConnectStore } from '~/stores/ConnectStore';
import { setPlaybackStore } from '~/stores/PlaybackStore';
import QueueList from './QueueList';

vi.mock('@iconify-icon/solid', () => ({
  Icon: () => null,
}));

vi.mock('@atlaskit/pragmatic-drag-and-drop/combine', () => ({
  combine: () => () => {},
}));

vi.mock('@atlaskit/pragmatic-drag-and-drop/element/adapter', () => ({
  draggable: () => () => {},
  dropTargetForElements: () => () => {},
}));

vi.mock('~/features/menu', () => ({
  buildQueueItemMenuItems: () => [],
}));

vi.mock('~/features/menu/useContextMenu', () => ({
  default: () => ({}),
}));

vi.mock('~/features/navigation/useSPNavigate', () => ({
  useSPNavigate: () => vi.fn(),
}));

vi.mock('~/bindings', () => ({
  commands: {
    playbackMoveQueueIndex: vi.fn().mockResolvedValue({ status: 'ok' }),
    playbackPlayQueueIndex: vi.fn(),
  },
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
  return {
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
    ...overrides,
  };
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
    updatedAt: '2026-06-06T00:00:00Z',
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

describe('QueueList', () => {
  let container: HTMLDivElement;
  let dispose: () => void;

  beforeEach(() => {
    resetStores();
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    dispose?.();
    container.remove();
    vi.clearAllMocks();
  });

  it('highlights the shared current queue row when the local queue index is stale', () => {
    const remoteQueue = [song('remote-1'), song('remote-2')];
    setPlaybackStore('status', playbackStatus([remoteQueue[1]], 0));
    setConnectStore('runtime', {
      enabled: true,
      connected: true,
      message: null,
      deviceId: 'own-device',
      seq: 1,
    });
    setConnectStore('sharedPlayback', sharedPlayback(remoteQueue, 1));
    setConnectStore('sharedPlaybackReceivedAtMs', 100);

    dispose = render(() => <QueueList queue={remoteQueue} />, container);

    const rows = container.querySelectorAll('.song-item');
    expect(rows[0].getAttribute('data-current')).toBeNull();
    expect(rows[1].getAttribute('data-current')).toBe('true');
  });
});
