import { commands, events, PlaybackStatus } from '~/bindings';
import { setPlaybackStore } from '~/stores/PlaybackStore';

type PlaybackCommandResult = { status: 'ok' } | { status: 'error'; error: string };

function applyPlaybackState(status: PlaybackStatus) {
  setPlaybackStore('status', status);
}

export async function startPlaybackStateSync() {
  let receivedEvent = false;
  const unlisten = await events.playbackStatus.listen((event) => {
    receivedEvent = true;
    applyPlaybackState(event.payload);
  });

  const stateResult = await commands.playbackGetState();
  if (stateResult.status === 'error') {
    console.error(stateResult.error);
    return unlisten;
  }

  if (!receivedEvent) {
    applyPlaybackState(stateResult.data);
  }

  return unlisten;
}

export function hasPlaybackCommandError(result: PlaybackCommandResult) {
  if (result.status === 'error') {
    console.error(result.error);
    return true;
  }

  return false;
}
