import { commands, events, PlaybackStatus } from '~/bindings';
import { playbackStore, setPlaybackStore } from '~/stores/PlaybackStore';

const PLAYBACK_STATE_POLL_MS = 1000;

type PlaybackCommandResult = { status: 'ok' } | { status: 'error'; error: string };

function applyPlaybackState(status: PlaybackStatus) {
  setPlaybackStore('status', status);
}

export async function startPlaybackStateSync() {
  let stateGeneration = 0;
  let pollInFlight = false;

  const applyAuthoritativeState = (status: PlaybackStatus) => {
    stateGeneration += 1;
    applyPlaybackState(status);
  };

  const refreshIfIdle = () => {
    if (pollInFlight) {
      return;
    }

    pollInFlight = true;
    void refreshPlaybackState().finally(() => {
      pollInFlight = false;
    });
  };

  const unlistenStatus = await events.playbackStatus.listen((event) => {
    applyAuthoritativeState(event.payload);
  });

  const unlistenNativeDirty = await events.playbackNativeDirty.listen(() => {
    refreshIfIdle();
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

  await refreshPlaybackState();

  const intervalId = window.setInterval(() => {
    if (pollInFlight || playbackStore.status?.state !== 'playing') {
      return;
    }

    refreshIfIdle();
  }, PLAYBACK_STATE_POLL_MS);

  return () => {
    window.clearInterval(intervalId);
    unlistenStatus();
    unlistenNativeDirty();
  };
}

export function hasPlaybackCommandError(result: PlaybackCommandResult) {
  if (result.status === 'error') {
    console.error(result.error);
    return true;
  }

  return false;
}
