import { createRoot } from 'solid-js';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { usePlayback } from '~/features/playback/usePlayback';
import { setSettingsStore } from '~/features/settings/service';
import { setConnectStore } from '~/stores/ConnectStore';
import { setPlaybackStore } from '~/stores/PlaybackStore';
import { resetConnectStore, setSharedPlayback, sharedPlayback } from '~test/fixtures/connect';
import { actualStreamInfo, playbackStatus, resetPlaybackStore, song } from '~test/fixtures/playback';
import { appSettings } from '~test/fixtures/settings';

const bindingMocks = vi.hoisted(() => ({
  playbackSetVolume: vi.fn(),
  settingsUpdate: vi.fn(),
}));

vi.mock('~/bindings', () => ({
  commands: {
    playbackSetVolume: bindingMocks.playbackSetVolume,
    settingsUpdate: bindingMocks.settingsUpdate,
  },
}));

describe('usePlayback', () => {
  beforeEach(() => {
    bindingMocks.playbackSetVolume.mockResolvedValue({ status: 'ok', data: null });
    bindingMocks.settingsUpdate.mockImplementation(async ({ settings }) => ({ status: 'ok', data: settings }));
    vi.clearAllMocks();
    resetPlaybackStore();
    setSettingsStore(appSettings({ connect: { deviceId: '' } }));
    resetConnectStore();
  });

  it('reads the local playback queue when Connect shared playback is unavailable', () => {
    const localQueue = [song('local-1'), song('local-2')];
    setPlaybackStore('status', playbackStatus({ queue: localQueue, currentIndex: 1 }));

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
    setPlaybackStore('status', playbackStatus({ queue: localQueue, currentIndex: 0 }));
    setSharedPlayback(sharedPlayback({ queue: remoteQueue, currentIndex: 1 }));

    const dispose = createRoot((dispose) => {
      const playback = usePlayback();

      expect(playback.queue().map((entry) => entry.id)).toEqual(['remote-1', 'remote-2']);
      expect(playback.currentIndex()).toBe(1);
      expect(playback.currentEntry()?.id).toBe('remote-2');

      return dispose;
    });

    dispose();
  });

  it('uses the refreshed Connect receive time when a same-seq report moves to the next track', () => {
    const remoteQueue = [song('remote-1'), song('remote-2')];
    const previousSharedPlayback = sharedPlayback({ queue: remoteQueue, currentIndex: 0 });
    const nextSharedPlayback = sharedPlayback({ queue: remoteQueue, currentIndex: 1, updatedAt: '2026-05-31T00:00:18Z' });
    setSharedPlayback(previousSharedPlayback);
    setPlaybackStore('clockMs', 18_100);

    const dispose = createRoot((dispose) => {
      const playback = usePlayback();

      expect(playback.currentPositionMs()).toBe(18_000);

      setConnectStore('sharedPlayback', nextSharedPlayback);
      setConnectStore('sharedPlaybackReceivedAtMs', 18_100);

      expect(playback.currentIndex()).toBe(1);
      expect(playback.currentEntry()?.id).toBe('remote-2');
      expect(playback.currentPositionMs()).toBe(0);

      return dispose;
    });

    dispose();
  });

  it('omits local playback details while remote Connect shared playback provides the display state', () => {
    const localQueue = [song('local-1')];
    const remoteQueue = [song('remote-1'), song('remote-2')];
    const localStatus = playbackStatus({
      queue: localQueue,
      currentIndex: 0,
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
      actualStreamInfo: actualStreamInfo('local-codec'),
    });
    const remoteSharedPlayback = sharedPlayback({ queue: remoteQueue, currentIndex: 1 });
    setPlaybackStore('status', localStatus);
    setSharedPlayback(remoteSharedPlayback);

    const dispose = createRoot((dispose) => {
      const playback = usePlayback();

      expect(playback.queue().map((entry) => entry.id)).toEqual(['remote-1', 'remote-2']);
      expect(playback.currentIndex()).toBe(1);
      expect(playback.interruptReason()).toBeNull();
      expect(playback.currentPositionMs()).toBe(0);
      expect(playback.gaplessStatus()?.state).toBe('unavailable');

      return dispose;
    });

    dispose();
  });

  it('commits a previewed volume without sending the volume command again', () => {
    const dispose = createRoot((dispose) => {
      const playback = usePlayback();

      playback.previewVolume(0.42);
      playback.commitPreviewedVolume(0.42);

      expect(bindingMocks.playbackSetVolume).toHaveBeenCalledTimes(1);
      expect(bindingMocks.playbackSetVolume).toHaveBeenCalledWith({ volume: 0.42 });
      expect(bindingMocks.settingsUpdate).toHaveBeenCalledTimes(1);
      expect(bindingMocks.settingsUpdate.mock.calls[0][0].settings.playback.volume).toBe(0.42);
      expect(playback.volume()).toBe(0.42);

      return dispose;
    });

    dispose();
  });
});
