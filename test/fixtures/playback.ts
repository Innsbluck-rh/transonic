import type { PlaybackActualStreamInfo, PlaybackStatus, SongResponse } from '~/bindings';
import { setPlaybackStore } from '~/stores/PlaybackStore';

export function song(id: string, title = id): SongResponse {
  return {
    id,
    parentId: null,
    path: null,
    title,
    album: null,
    albumId: null,
    artist: null,
    artistId: null,
    coverArtId: null,
    displayCoverArtId: null,
    track: null,
    discNumber: null,
    year: null,
    duration: null,
    size: null,
    contentType: null,
    suffix: null,
    bitRate: null,
    genre: null,
    created: null,
    starred: null,
    isDirectory: false,
    mediaType: 'song',
  };
}

// playingState and currentSongId follow queue/currentIndex unless overridden.
export function playbackStatus(overrides: Partial<PlaybackStatus> = {}): PlaybackStatus {
  const queue = overrides.queue ?? [];
  const currentIndex = overrides.currentIndex ?? null;
  return {
    playingState: queue.length ? 'playing' : 'idle',
    interruptReason: null,
    pendingSeekPositionMs: null,
    gaplessStatus: {
      state: 'unavailable',
      message: 'gapless: unavailable',
    },
    queue,
    currentIndex,
    playNextQueueLen: 0,
    currentPositionMs: 0,
    currentSongId: currentIndex === null ? null : (queue[currentIndex]?.id ?? null),
    outputHost: null,
    playbackError: null,
    activeStreamInfo: null,
    preparedStreamInfo: null,
    lastStreamRequest: null,
    actualStreamInfo: null,
    ...overrides,
  };
}

export function actualStreamInfo(codec: string): PlaybackActualStreamInfo {
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

export function resetPlaybackStore() {
  setPlaybackStore('status', undefined);
  setPlaybackStore('authoritativeReceivedAtMs', null);
  setPlaybackStore('clockMs', 0);
}
