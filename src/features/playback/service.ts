import { commands, events, type PlaybackStatus } from '~/bindings';
import { connectStore } from '~/stores/ConnectStore';
import { playbackStore, setPlaybackStore } from '~/stores/PlaybackStore';
import { resolvePlaybackDisplayStatus } from './sharedPlaybackStatus';

type PlaybackCommandResult = { status: 'ok' } | { status: 'error'; error: string };

let requestPlaybackStateRefresh: (() => void) | undefined;

function monotonicNowMs() {
  return typeof performance === 'undefined' ? Date.now() : performance.now();
}

function applyPlaybackState(status: PlaybackStatus, receivedAtMs: number) {
  setPlaybackStore({
    status,
    authoritativeReceivedAtMs: receivedAtMs,
    clockMs: receivedAtMs,
  });
}

export async function startPlaybackStateSync() {
  let stateGeneration = 0;
  let refreshInFlight = false;
  let animationFrameId: number | undefined;
  let latestSharedPlaybackPlaying = false;

  const applyAuthoritativeState = (status: PlaybackStatus) => {
    stateGeneration += 1;
    applyPlaybackState(status, monotonicNowMs());
    syncClockLoop();
  };

  const refreshIfIdle = () => {
    if (refreshInFlight) {
      return;
    }

    refreshInFlight = true;
    void refreshPlaybackState().finally(() => {
      refreshInFlight = false;
    });
  };

  const stopClockLoop = () => {
    if (animationFrameId === undefined) {
      return;
    }

    window.cancelAnimationFrame(animationFrameId);
    animationFrameId = undefined;
  };

  const displayStatus = () =>
    resolvePlaybackDisplayStatus(playbackStore.status, connectStore.runtime.connected, connectStore.sharedPlayback, connectStore.runtime.deviceId);
  const shouldTickClock = () => (displayStatus()?.playingState === 'playing' || latestSharedPlaybackPlaying) && !document.hidden;

  const startClockLoop = () => {
    if (animationFrameId !== undefined) {
      return;
    }

    animationFrameId = window.requestAnimationFrame(tickClock);
  };

  const tickClock = () => {
    if (!shouldTickClock()) {
      animationFrameId = undefined;
      return;
    }

    setPlaybackStore('clockMs', monotonicNowMs());
    animationFrameId = undefined;
    startClockLoop();
  };

  const syncClockLoop = () => {
    if (!shouldTickClock()) {
      stopClockLoop();
      return;
    }

    startClockLoop();
  };

  const unlistenStatus = await events.playbackStatus.listen((event) => {
    applyAuthoritativeState(event.payload);
  });
  const unlistenConnectState = await events.connectStateUpdated.listen((event) => {
    latestSharedPlaybackPlaying = event.payload.sharedPlayback?.state.playingState === 'playing';
    setPlaybackStore('clockMs', monotonicNowMs());
    syncClockLoop();
  });

  const refreshPlaybackState = async () => {
    const startedAtGeneration = stateGeneration;
    const stateResult = await commands.playbackGetState();
    if (stateResult.status === 'error') {
      console.error(stateResult.error);
      return;
    }

    if (startedAtGeneration !== stateGeneration) {
      return;
    }

    applyAuthoritativeState(stateResult.data);
  };

  const refreshAfterResume = () => {
    if (document.hidden) {
      stopClockLoop();
      return;
    }

    refreshIfIdle();
    syncClockLoop();
  };

  requestPlaybackStateRefresh = refreshIfIdle;

  await refreshPlaybackState();
  window.addEventListener('focus', refreshAfterResume);
  document.addEventListener('visibilitychange', refreshAfterResume);
  syncClockLoop();

  return () => {
    stopClockLoop();
    window.removeEventListener('focus', refreshAfterResume);
    document.removeEventListener('visibilitychange', refreshAfterResume);
    if (requestPlaybackStateRefresh === refreshIfIdle) {
      requestPlaybackStateRefresh = undefined;
    }
    unlistenStatus();
    unlistenConnectState();
  };
}

export function hasPlaybackCommandError(result: PlaybackCommandResult) {
  if (result.status === 'error') {
    console.error(result.error);
    requestPlaybackStateRefresh?.();
    return true;
  }

  return false;
}
