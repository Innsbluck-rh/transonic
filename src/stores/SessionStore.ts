import { createStore } from 'solid-js/store';
import type { ActiveSession, SavedProfileSummary } from '~/bindings';

interface SessionStore {
  profiles: SavedProfileSummary[];
  activeSession: ActiveSession | null;
}

const DEFAULT_SESSION_STORE: SessionStore = {
  profiles: [],
  activeSession: null,
} as const;

const [sessionStore, setSessionStore] = createStore<SessionStore>(DEFAULT_SESSION_STORE);

export { sessionStore, setSessionStore };
