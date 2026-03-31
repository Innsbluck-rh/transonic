import { type AppBootstrap } from '~/bindings';
import { sessionStore, setSessionStore } from '~/stores/SessionStore';

export function getProfileById(profileId: string) {
  return sessionStore.profiles.find((p) => p.profileId === profileId);
}

export async function loadBootstrapToStore(bootstrap: AppBootstrap) {
  setSessionStore('profiles', bootstrap.profiles);
  setSessionStore('activeSession', bootstrap.activeSession);
}
