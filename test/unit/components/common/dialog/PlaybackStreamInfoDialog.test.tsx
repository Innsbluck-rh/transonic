import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { PlaybackStreamInfo } from '~/bindings';
import PlaybackStreamInfoDialog, {
  closePlaybackStreamInfoDialog,
  openPlaybackStreamInfoDialog,
} from '~/components/common/dialog/PlaybackStreamInfoDialog';
import { setSettingsStore } from '~/features/settings/service';
import { setPlaybackStore } from '~/stores/PlaybackStore';
import { resetConnectStore, setSharedPlayback, sharedPlayback } from '~test/fixtures/connect';
import { actualStreamInfo, playbackStatus, resetPlaybackStore, song } from '~test/fixtures/playback';
import { appSettings } from '~test/fixtures/settings';

vi.mock('@iconify-icon/solid', () => ({
  Icon: () => null,
}));

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

describe('PlaybackStreamInfoDialog', () => {
  let container: HTMLDivElement;
  let dispose: () => void;

  beforeEach(() => {
    closePlaybackStreamInfoDialog();
    resetPlaybackStore();
    resetConnectStore();
    setSettingsStore(appSettings({ playback: { gaplessPlaybackEnabled: true }, connect: { deviceId: '' } }));
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
        queue: [song('local-song')],
        currentIndex: 0,
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
    setSharedPlayback(sharedPlayback({ queue: [song('remote-song')], currentIndex: 0 }));

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
