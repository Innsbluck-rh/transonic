import { convertFileSrc } from '@tauri-apps/api/core';
import { commands, type CoverArtRequest } from '~/bindings';

export interface CoverArtResourceRequest extends CoverArtRequest {
  profileId: string;
}

export async function fetchCoverArtAssetUrl(payload: CoverArtResourceRequest) {
  try {
    const response = await commands.getCoverArt({
      coverArtId: payload.coverArtId,
      size: payload.size,
    });
    if (response.status === 'error') {
      throw new Error(response.error);
    }

    return convertFileSrc(response.data.localPath);
  } catch (invokeError) {
    console.error(invokeError);
    return null;
  }
}
