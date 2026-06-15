// @vitest-environment jsdom

import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { ConnectSharedPlaybackState, PlaybackActualStreamInfo, PlaybackStatus } from '~/bindings';
import { setConnectStore } from '~/stores/ConnectStore';
import { setPlaybackStore } from '~/stores/PlaybackStore';
import PlaybackStatusText from './PlaybackStatusText';

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

function playbackStatus(overrides: Partial<PlaybackStatus> = {}): PlaybackStatus {
  return {
    playingState: 'playing',
    interruptReason: null,
    pendingSeekPositionMs: null,
    gaplessStatus: {
      state: 'unavailable',
      message: 'gapless: unavailable',
    },
    queue: [],
    currentIndex: null,
    playNextQueueLen: 0,
    currentPositionMs: 0,
    currentSongId: null,
    playbackError: null,
    activeStreamInfo: null,
    preparedStreamInfo: null,
    lastStreamRequest: null,
    actualStreamInfo: null,
    ...overrides,
  };
}

function sharedPlayback(activeDeviceId: string): ConnectSharedPlaybackState {
  return {
    seq: 1,
    activeDeviceId,
    state: {
      playingState: 'playing',
      queue: [],
      currentIndex: null,
      playNextQueueLen: 0,
      currentPositionMs: 0,
      currentSongId: null,
    },
    updatedAt: '2026-06-13T00:00:00Z',
    updatedByDeviceId: activeDeviceId,
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

describe('PlaybackStatusText', () => {
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
  });

  it('hides local stream info while another Connect device is active', () => {
    setPlaybackStore('status', playbackStatus({ actualStreamInfo: actualStreamInfo('local-codec') }));
    setConnectStore('runtime', {
      enabled: true,
      connected: true,
      message: null,
      deviceId: 'own-device',
      seq: 1,
    });
    setConnectStore('sharedPlayback', sharedPlayback('remote-device'));
    setConnectStore('sharedPlaybackReceivedAtMs', 100);

    dispose = render(() => <PlaybackStatusText />, container);

    expect(container.textContent).not.toContain('local-codec');
    expect(container.textContent).toBe('');
  });

  it('shows local stream info when this Connect device is active', () => {
    setPlaybackStore('status', playbackStatus({ actualStreamInfo: actualStreamInfo('local-codec') }));
    setConnectStore('runtime', {
      enabled: true,
      connected: true,
      message: null,
      deviceId: 'own-device',
      seq: 1,
    });
    setConnectStore('sharedPlayback', sharedPlayback('own-device'));
    setConnectStore('sharedPlaybackReceivedAtMs', 100);

    dispose = render(() => <PlaybackStatusText />, container);

    expect(container.textContent).toContain('local-codec');
  });
});
