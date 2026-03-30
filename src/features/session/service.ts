import { commands, type AppBootstrap } from '~/bindings';
import { syncFolderStructureSession } from '~/features/browse/folderStructureService';
import { sessionStore, setSessionStore } from '~/stores/session/SessionStore';

export function getProfileById(profileId: string) {
  return sessionStore.profiles.find((p) => p.profileId === profileId);
}

export async function fetchBootstrapAppState() {
  const result = await commands.bootstrapAppState();
  if (result.status === 'error') {
    throw new Error(result.error);
  }

  return result.data;
}

export async function loadBootstrapToStore(bootstrap: AppBootstrap) {
  syncFolderStructureSession(bootstrap.activeSession?.profileId ?? null);
  setSessionStore('profiles', bootstrap.profiles);
  setSessionStore('activeSession', bootstrap.activeSession);
}
