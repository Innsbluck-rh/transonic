import { commands } from '~/bindings';
import { getProfileById, loadBootstrapToStore } from './service';

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
    const result = await commands.deleteServerProfile({
      profileId,
    });
    if (result.status === 'error') {
      throw new Error(result.error);
    }

    await loadBootstrapToStore(result.data);
  } catch (invokeError) {
    console.error(invokeError);
  }
}
