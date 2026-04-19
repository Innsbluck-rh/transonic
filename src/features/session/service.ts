import { type AppBootstrap } from '~/bindings';
import { hydrateSettings } from '~/features/settings/service';
import { sessionStore, setSessionStore } from '~/stores/SessionStore';

export function getProfileById(profileId: string) {
  return sessionStore.profiles.find((p) => p.profileId === profileId);
}

export async function loadBootstrapToStore(bootstrap: AppBootstrap) {
  setSessionStore('profiles', bootstrap.profiles);
  setSessionStore('activeSession', bootstrap.activeSession);
  setSessionStore('playbackCapabilities', bootstrap.playbackCapabilities);
  await hydrateSettings(bootstrap.settings, bootstrap.settingsOrigin);
}
