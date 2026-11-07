import { describe, expect, it } from 'vitest';

import { isCurrentSong } from '~/features/playback/currentSong';
import { playbackStatus } from '~test/fixtures/playback';

describe('isCurrentSong', () => {
  it('matches the current song by song id when queue index is not provided', () => {
    expect(isCurrentSong({ playbackStatus: playbackStatus({ currentSongId: 'song-1' }), songId: 'song-1' })).toBe(true);
  });

  it('requires both song id and queue index to match for queue rows', () => {
    const status = playbackStatus({ currentSongId: 'song-1', currentIndex: 4 });

    expect(isCurrentSong({ playbackStatus: status, songId: 'song-1', queueIndex: 4 })).toBe(true);
    expect(isCurrentSong({ playbackStatus: status, songId: 'song-1', queueIndex: 3 })).toBe(false);
  });
});
