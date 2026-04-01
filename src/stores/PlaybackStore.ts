import { createStore } from 'solid-js/store';
import { PlaybackStatus } from '~/bindings';

interface PlaybackStore {
  status?: PlaybackStatus;
}

export const [playbackStore, setPlaybackStore] = createStore<PlaybackStore>();
