import { commands, PlaybackStatus, Result } from '~/bindings';
import { setPlaybackStore } from '~/stores/PlaybackStore';

// TODO: !! consider replacing calling this with listening the backend events !!
export async function syncPlaybackState() {
  const stateResult = await commands.playbackGetState();

  if (stateResult.status === 'error') {
    console.error(stateResult.error);
    return;
  }
  setPlaybackStore('status', stateResult.data);
}

export async function setPlaybackStateWithResult(result: Result<PlaybackStatus, string>) {
  if (result.status === 'error') {
    console.error(result.error);
    return;
  }
  setPlaybackStore('status', result.data);
}
