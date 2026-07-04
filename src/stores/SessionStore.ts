import { createStore } from 'solid-js/store';
import type { ActiveSession, PlaybackCapabilities, SavedProfileSummary } from '~/bindings';

interface SessionStore {
  profiles: SavedProfileSummary[];
  activeSession: ActiveSession | null;
  playbackCapabilities: PlaybackCapabilities;
}

const DEFAULT_SESSION_STORE: SessionStore = {
  profiles: [],
  activeSession: null,
  playbackCapabilities: {
    gaplessPlayback: false,
    transcodingCodecs: ['mp3'],
    outputDeviceSelection: false,
  },
};

export const [sessionStore, setSessionStore] = createStore<SessionStore>(DEFAULT_SESSION_STORE);
