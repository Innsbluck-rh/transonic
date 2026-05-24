import { describe, expect, it } from 'vitest';

import type { PlaybackStatus } from '~/bindings';

import { resolveSongItemConditions, songItemConditionAttrs } from './SongItemConditions';

function createPlaybackStatus(overrides: Partial<PlaybackStatus> = {}): PlaybackStatus {
  return {
    playingState: 'idle',
    interruptReason: null,
    pendingSeekPositionMs: null,
    gaplessStatus: { state: 'idle', message: '' },
    queue: [],
    currentIndex: null,
    currentPositionMs: 0,
    currentSongId: null,
    playbackError: null,
    ...overrides,
  };
}

describe('song item conditions', () => {
  it('marks the current song by song id when queue index is not provided', () => {
    const conditions = resolveSongItemConditions({
      playbackStatus: createPlaybackStatus({ currentSongId: 'song-1' }),
      songId: 'song-1',
    });

    expect(conditions).toEqual({
      current: true,
      selected: false,
    });
  });

  it('requires both song id and queue index to match for queue rows', () => {
    const playbackStatus = createPlaybackStatus({
      currentSongId: 'song-1',
      currentIndex: 4,
    });

    expect(
      resolveSongItemConditions({
        playbackStatus,
        songId: 'song-1',
        queueIndex: 4,
      }).current
    ).toBe(true);

    expect(
      resolveSongItemConditions({
        playbackStatus,
        songId: 'song-1',
        queueIndex: 3,
      }).current
    ).toBe(false);
  });

  it('serializes active conditions into data attributes', () => {
    expect(
      songItemConditionAttrs({
        current: true,
        selected: false,
      })
    ).toEqual({
      'data-current': 'true',
      'data-selected': undefined,
    });
  });
});
