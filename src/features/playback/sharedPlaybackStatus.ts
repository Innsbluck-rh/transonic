import type { ConnectSharedPlaybackState, PlaybackStatus } from '~/bindings';

function emptyPlaybackStatus(): PlaybackStatus {
  return {
    playingState: 'idle',
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
    playbackError: null,
    activeStreamInfo: null,
    preparedStreamInfo: null,
    lastStreamRequest: null,
    actualStreamInfo: null,
  };
}

export function mergePlaybackStatusWithSharedPlayback(
  baseStatus: PlaybackStatus | undefined,
  sharedPlayback: ConnectSharedPlaybackState
): PlaybackStatus {
  const state = sharedPlayback.state;
  const base = baseStatus ?? emptyPlaybackStatus();

  return {
    ...base,
    playingState: state.playingState,
    queue: state.queue ?? base.queue,
    currentIndex: state.currentIndex === undefined ? base.currentIndex : state.currentIndex,
    playNextQueueLen: state.playNextQueueLen ?? base.playNextQueueLen,
    currentPositionMs: state.currentPositionMs ?? base.currentPositionMs,
    currentSongId: state.currentSongId === undefined ? base.currentSongId : state.currentSongId,
  };
}
