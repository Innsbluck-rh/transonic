import { invoke } from "@tauri-apps/api/core";
import { getProfileById, loadBootstrapToStore } from "./service";
import { AppBootstrap } from "~/models/bootstrap";

export async function deleteProfile(profileId: string) {
  const profile = getProfileById(profileId);
  if (profile == null) {
    return;
  }
  const confirmed = window.confirm(`Delete profile "${profile.displayName}"?`);
  if (!confirmed) {
    return;
  }
  try {
    const bootstrap = await invoke<AppBootstrap>('delete_server_profile', {
      payload: {
        profileId,
      },
    });
    loadBootstrapToStore(bootstrap);
  } catch (invokeError) {
    console.error(invokeError);
  }
}
