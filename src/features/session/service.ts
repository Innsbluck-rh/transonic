import { AppBootstrap } from "~/models/bootstrap";
import { ConnectServerProfileResult } from "~/routes/init_login";
import { sessionStore, setSessionStore } from "~/stores/session/SessionStore";

export function getProfileById(profileId: string) {
  return sessionStore.profiles.find((p) => p.profileId === profileId);
}

export async function loadBootstrapToStore(bootstrap: AppBootstrap) {
  setSessionStore("profiles", bootstrap.profiles);
  setSessionStore("activeSession", bootstrap.activeSession);
}
