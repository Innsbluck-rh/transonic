// @vitest-environment jsdom

import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ConnectSharedPlaybackState, PlaybackActualStreamInfo, PlaybackStatus } from '~/bindings';
// The ASIO marker links to the playback settings, and resolving that route asks
// the OS plugin whether this is a smartphone layout.
vi.mock('@tauri-apps/plugin-os', () => ({
  platform: () => 'windows',
}));

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
    outputHost: null,
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

  it('marks ASIO output and leaves other output paths unmarked', () => {
    setPlaybackStore('status', playbackStatus({ actualStreamInfo: actualStreamInfo('flac') }));
    dispose = render(() => <PlaybackStatusText />, container);
    expect(container.textContent).toContain('flac');
    expect(container.textContent).not.toContain('ASIO');

    setPlaybackStore('status', playbackStatus({ actualStreamInfo: actualStreamInfo('flac'), outputHost: 'wasapi' }));
    expect(container.textContent).not.toContain('ASIO');

    setPlaybackStore('status', playbackStatus({ actualStreamInfo: actualStreamInfo('flac'), outputHost: 'asio' }));
    const asioLabel = Array.from(container.querySelectorAll('*')).find((element) => element.textContent === 'ASIO');
    expect(asioLabel).toBeDefined();
    expect(asioLabel!.className).toContain('text-accent');
    expect(asioLabel!.className).toContain('font-bold');
    // The marker doubles as the way to reach the setting that produced it.
    expect(asioLabel!.getAttribute('href')).toBe('/settings#playback');
  });

  it('does not mark ASIO output while another Connect device is active', () => {
    // The output path is a fact about this machine, so it must not be claimed
    // for audio that is coming out of a different device.
    setPlaybackStore('status', playbackStatus({ actualStreamInfo: actualStreamInfo('flac'), outputHost: 'asio' }));
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

    expect(container.textContent).not.toContain('ASIO');
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
