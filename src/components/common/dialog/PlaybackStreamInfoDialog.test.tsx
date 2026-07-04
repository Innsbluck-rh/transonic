// @vitest-environment jsdom

import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ConnectSharedPlaybackState, PlaybackActualStreamInfo, PlaybackStatus, PlaybackStreamInfo, SongResponse } from '~/bindings';
import { setSettingsStore } from '~/features/settings/service';
import { setConnectStore } from '~/stores/ConnectStore';
import { setPlaybackStore } from '~/stores/PlaybackStore';
import PlaybackStreamInfoDialog, { closePlaybackStreamInfoDialog, openPlaybackStreamInfoDialog } from './PlaybackStreamInfoDialog';

vi.mock('@iconify-icon/solid', () => ({
  Icon: () => null,
}));

function song(id: string): SongResponse {
  return {
    id,
    parentId: null,
    path: null,
    title: id,
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

function actualStreamInfo(codec: string): PlaybackActualStreamInfo {
  return {
    codec,
    codecProfile: null,
    sampleRate: 44100,
    channels: 2,
    bitDepth: 16,
    bitrate: 320000,
    sampleFormat: null,
    mimeType: null,
  };
}

function streamInfo(streamUrl: string): PlaybackStreamInfo {
  return {
    requestKind: 'load',
    songId: 'local-song',
    songPath: '/music/local-song.flac',
    sourceContentType: 'audio/flac',
    sourceSuffix: 'flac',
    sourceBitRate: 320,
    sourceSize: 12345,
    streamMode: 'raw',
    rawStream: true,
    streamFormat: null,
    streamMaxBitRate: null,
    streamOffsetSeconds: null,
    requestedPositionMs: 0,
    localStartPositionMs: 0,
    autoplay: true,
    streamEndpoint: 'stream',
    streamUrl,
  };
}

function playbackStatus(overrides: Partial<PlaybackStatus> = {}): PlaybackStatus {
  return {
    playingState: 'playing',
    interruptReason: null,
    pendingSeekPositionMs: null,
    gaplessStatus: {
      state: 'unavailable',
      message: 'gapless: unavailable',
    },
    queue: [song('local-song')],
    currentIndex: 0,
    playNextQueueLen: 0,
    currentPositionMs: 0,
    currentSongId: 'local-song',
    playbackError: null,
    activeStreamInfo: null,
    preparedStreamInfo: null,
    lastStreamRequest: null,
    actualStreamInfo: null,
    ...overrides,
  };
}

function sharedPlayback(): ConnectSharedPlaybackState {
  const remoteSong = song('remote-song');
  return {
    seq: 1,
    activeDeviceId: 'remote-device',
    state: {
      playingState: 'playing',
      queue: [remoteSong],
      currentIndex: 0,
      playNextQueueLen: 0,
      currentPositionMs: 0,
      currentSongId: remoteSong.id,
    },
    updatedAt: '2026-06-13T00:00:00Z',
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
  setSettingsStore({
    appearance: {
      albumDisplayMode: 'grid',
    },
    playback: {
      gaplessPlaybackEnabled: true,
      volume: 1,
      useCustomOutput: false,
      outputDeviceId: null,
      streamMode: 'raw',
      meteredNetworkTranscodingEnabled: false,
      transcodingBitrateLimit: 320,
      useCustomTranscodingCodec: false,
      transcodingCodec: 'auto',
    },
    connect: {
      enabled: false,
      useSubsonicServerHost: true,
      connectServerPort: 4747,
      connectServerHost: null,
      deviceId: '',
      deviceName: null,
      allowInsecureConnectServer: false,
    },
  });
}

describe('PlaybackStreamInfoDialog', () => {
  let container: HTMLDivElement;
  let dispose: () => void;

  beforeEach(() => {
    closePlaybackStreamInfoDialog();
    resetStores();
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    closePlaybackStreamInfoDialog();
    dispose?.();
    container.remove();
    vi.clearAllMocks();
  });

  it('does not show local playback diagnostics while another Connect device is active', () => {
    setPlaybackStore(
      'status',
      playbackStatus({
        interruptReason: 'initial_load',
        gaplessStatus: {
          state: 'preparing',
          message: 'gapless: preparing',
        },
        actualStreamInfo: actualStreamInfo('local-codec'),
        activeStreamInfo: streamInfo('https://example.test/local-stream'),
        preparedStreamInfo: streamInfo('https://example.test/local-prepared-stream'),
        lastStreamRequest: streamInfo('https://example.test/local-last-stream'),
      })
    );
    setConnectStore('runtime', {
      enabled: true,
      connected: true,
      message: null,
      deviceId: 'own-device',
      seq: 1,
    });
    setConnectStore('sharedPlayback', sharedPlayback());
    setConnectStore('sharedPlaybackReceivedAtMs', 100);

    openPlaybackStreamInfoDialog();
    dispose = render(() => <PlaybackStreamInfoDialog />, container);

    const text = container.textContent ?? '';
    expect(text).not.toContain('initial_load');
    expect(text).not.toContain('gapless: preparing');
    expect(text).not.toContain('local-codec');
    expect(text).not.toContain('local-stream');
    expect(text).toContain('gapless: remote playback diagnostics unavailable');
    expect(text).toContain('unavailable for remote playback');
  });
});
