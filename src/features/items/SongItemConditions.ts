import type { PlaybackStatus } from '~/bindings';

export interface SongItemConditions {
  current: boolean;
  selected: boolean;
}

interface ResolveSongItemConditionsOptions {
  playbackStatus?: PlaybackStatus;
  songId: string;
  queueIndex?: number;
  selected?: boolean;
}

export function resolveSongItemConditions(options: ResolveSongItemConditionsOptions): SongItemConditions {
  const currentSongId = options.playbackStatus?.currentSongId;
  const currentIndex = options.playbackStatus?.currentIndex;
  const matchesSong = currentSongId === options.songId;
  const matchesQueueIndex = options.queueIndex === undefined || currentIndex === options.queueIndex;

  return {
    current: matchesSong && matchesQueueIndex,
    selected: options.selected ?? false,
  };
}

export function songItemConditionAttrs(conditions: SongItemConditions) {
  return {
    'data-current': conditions.current ? 'true' : undefined,
    'data-selected': conditions.selected ? 'true' : undefined,
  };
}
