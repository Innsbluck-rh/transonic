import { describe, expect, it } from 'vitest';

import { resolveSongItemConditions, songItemConditionAttrs } from '~/features/items/SongItemConditions';
import { playbackStatus } from '~test/fixtures/playback';

describe('song item conditions', () => {
  it('marks the current song by song id when queue index is not provided', () => {
    const conditions = resolveSongItemConditions({
      playbackStatus: playbackStatus({ currentSongId: 'song-1' }),
      songId: 'song-1',
    });

    expect(conditions).toEqual({
      current: true,
      selected: false,
    });
  });

  it('requires both song id and queue index to match for queue rows', () => {
    const status = playbackStatus({
      currentSongId: 'song-1',
      currentIndex: 4,
    });

    expect(
      resolveSongItemConditions({
        playbackStatus: status,
        songId: 'song-1',
        queueIndex: 4,
      }).current
    ).toBe(true);

    expect(
      resolveSongItemConditions({
        playbackStatus: status,
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
