import { createStore } from 'solid-js/store';
import { PlaybackStatus } from '~/bindings';

interface PlaybackStore {
  status?: PlaybackStatus;
  authoritativeReceivedAtMs: number | null;
  clockMs: number;
}

const initialClockMs = typeof performance === 'undefined' ? 0 : performance.now();

export const [playbackStore, setPlaybackStore] = createStore<PlaybackStore>({
  authoritativeReceivedAtMs: null,
  clockMs: initialClockMs,
});
