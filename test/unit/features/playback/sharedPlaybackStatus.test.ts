import { describe, expect, it } from 'vitest';
import { mergePlaybackStatusWithSharedPlayback } from '~/features/playback/sharedPlaybackStatus';
import { sharedPlayback } from '~test/fixtures/connect';
import { actualStreamInfo, playbackStatus, song } from '~test/fixtures/playback';

describe('mergePlaybackStatusWithSharedPlayback', () => {
  const queue = [song('song-a')];
  const localStatus = playbackStatus({
    queue,
    currentIndex: 0,
    interruptReason: 'stream_buffering_stall',
    pendingSeekPositionMs: 12000,
    gaplessStatus: {
      state: 'ready',
      message: 'gapless: ready',
    },
    playbackError: {
      message: 'local error',
      handled: false,
    },
    actualStreamInfo: actualStreamInfo('local-codec'),
  });

  function merge(activeDeviceId: string | null) {
    return mergePlaybackStatusWithSharedPlayback(localStatus, sharedPlayback({ queue, currentIndex: 0, activeDeviceId }), 'own-device');
  }

  it('keeps local playback diagnostics while this device is active', () => {
    const merged = merge('own-device');

    expect(merged.interruptReason).toBe('stream_buffering_stall');
    expect(merged.pendingSeekPositionMs).toBe(12000);
    expect(merged.gaplessStatus.state).toBe('ready');
    expect(merged.playbackError?.message).toBe('local error');
    expect(merged.actualStreamInfo?.codec).toBe('local-codec');
  });

  it.each([
    ['another device is', 'remote-device'],
    ['no device is', null],
  ])('drops local playback diagnostics while %s active', (_, activeDeviceId) => {
    const merged = merge(activeDeviceId);

    expect(merged.interruptReason).toBeNull();
    expect(merged.pendingSeekPositionMs).toBeNull();
    expect(merged.gaplessStatus).toEqual({
      state: 'unavailable',
      message: 'gapless: remote playback diagnostics unavailable',
    });
    expect(merged.playbackError).toBeNull();
    expect(merged.actualStreamInfo).toBeNull();
  });
});
