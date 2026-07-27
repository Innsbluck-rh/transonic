import type { ConnectSharedPlaybackState, GaplessStatus, PlaybackStatus } from '~/bindings';

function unavailableGaplessStatus(message: string): GaplessStatus {
  return {
    state: 'unavailable',
    message,
  };
}

function emptyPlaybackStatus(): PlaybackStatus {
  return {
    playingState: 'idle',
    interruptReason: null,
    pendingSeekPositionMs: null,
    gaplessStatus: unavailableGaplessStatus('gapless: unavailable'),
    queue: [],
    currentIndex: null,
    playNextQueueLen: 0,
    currentPositionMs: 0,
    currentSongId: null,
    outputHost: null,
    playbackError: null,
    activeStreamInfo: null,
    preparedStreamInfo: null,
    lastStreamRequest: null,
    actualStreamInfo: null,
  };
}

// The output host describes this machine's audio path, so it belongs with the
// other diagnostics that are dropped while another device is the one playing.
function remotePlaybackDiagnosticsStatus(): Pick<
  PlaybackStatus,
  | 'interruptReason'
  | 'pendingSeekPositionMs'
  | 'gaplessStatus'
  | 'outputHost'
  | 'playbackError'
  | 'activeStreamInfo'
  | 'preparedStreamInfo'
  | 'lastStreamRequest'
  | 'actualStreamInfo'
> {
  return {
    interruptReason: null,
    pendingSeekPositionMs: null,
    gaplessStatus: unavailableGaplessStatus('gapless: remote playback diagnostics unavailable'),
    outputHost: null,
    playbackError: null,
    activeStreamInfo: null,
    preparedStreamInfo: null,
    lastStreamRequest: null,
    actualStreamInfo: null,
  };
}

export function usesLocalPlaybackDiagnostics(sharedPlayback: ConnectSharedPlaybackState | null | undefined, ownDeviceId?: string | null): boolean {
  if (!sharedPlayback) {
    return true;
  }

  return sharedPlayback.activeDeviceId !== null && sharedPlayback.activeDeviceId === ownDeviceId;
}

export function mergePlaybackStatusWithSharedPlayback(
  baseStatus: PlaybackStatus | undefined,
  sharedPlayback: ConnectSharedPlaybackState,
  ownDeviceId?: string | null
): PlaybackStatus {
  const state = sharedPlayback.state;
  const base = baseStatus ?? emptyPlaybackStatus();
  const preserveLocalPlaybackDiagnostics = usesLocalPlaybackDiagnostics(sharedPlayback, ownDeviceId);
  const stateFallback = preserveLocalPlaybackDiagnostics ? base : emptyPlaybackStatus();
  const diagnostics = preserveLocalPlaybackDiagnostics ? base : remotePlaybackDiagnosticsStatus();

  return {
    ...stateFallback,
    playingState: state.playingState,
    interruptReason: diagnostics.interruptReason,
    pendingSeekPositionMs: diagnostics.pendingSeekPositionMs,
    gaplessStatus: diagnostics.gaplessStatus,
    queue: state.queue ?? stateFallback.queue,
    currentIndex: state.currentIndex === undefined ? stateFallback.currentIndex : state.currentIndex,
    playNextQueueLen: state.playNextQueueLen ?? stateFallback.playNextQueueLen,
    currentPositionMs: state.currentPositionMs ?? stateFallback.currentPositionMs,
    currentSongId: state.currentSongId === undefined ? stateFallback.currentSongId : state.currentSongId,
    outputHost: diagnostics.outputHost,
    playbackError: diagnostics.playbackError,
    activeStreamInfo: diagnostics.activeStreamInfo,
    preparedStreamInfo: diagnostics.preparedStreamInfo,
    lastStreamRequest: diagnostics.lastStreamRequest,
    actualStreamInfo: diagnostics.actualStreamInfo,
  };
}

export function resolvePlaybackDisplayStatus(
  baseStatus: PlaybackStatus | undefined,
  connected: boolean,
  sharedPlayback: ConnectSharedPlaybackState | null,
  ownDeviceId?: string | null
): PlaybackStatus | undefined {
  if (!connected || !sharedPlayback) {
    return baseStatus;
  }

  return mergePlaybackStatusWithSharedPlayback(baseStatus, sharedPlayback, ownDeviceId);
}
