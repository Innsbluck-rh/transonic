import { convertFileSrc } from '@tauri-apps/api/core';
import { commands, type CoverArtRequest } from '~/bindings';

export type CoverArtResourceRequest = CoverArtRequest;

export async function fetchCoverArtAssetUrl(payload: CoverArtResourceRequest): Promise<string | null> {
  try {
    const response = await commands.getCoverArt(payload);
    if (response.status === 'error') {
      throw new Error(response.error);
    }

    return convertFileSrc(response.data.localPath);
  } catch (invokeError) {
    console.error(invokeError);
    return null;
  }
}
