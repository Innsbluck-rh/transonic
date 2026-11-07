import type { PlaybackStatus } from '~/bindings';

interface CurrentSongOptions {
  playbackStatus?: PlaybackStatus;
  songId: string;
  /** For queue rows: the same song can be queued twice, so the position has to match too. */
  queueIndex?: number;
}

/** Whether a song row is the one playing. */
export function isCurrentSong(options: CurrentSongOptions): boolean {
  const matchesSong = options.playbackStatus?.currentSongId === options.songId;
  const matchesQueueIndex = options.queueIndex === undefined || options.playbackStatus?.currentIndex === options.queueIndex;
  return matchesSong && matchesQueueIndex;
}
