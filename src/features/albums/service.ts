import { convertFileSrc } from '@tauri-apps/api/core';
import { commands, type CoverArtRequest } from '~/bindings';

export interface CoverArtResourceRequest extends CoverArtRequest {
  profileId: string;
}

const coverArtCache = new Map<string, Promise<string | null>>();

function coverArtCacheKey(payload: CoverArtResourceRequest): string {
  return `${payload.profileId}\0${payload.coverArtId}\0${payload.size ?? 'full'}`;
}

async function fetchCoverArtAssetUrlUncached(payload: CoverArtResourceRequest): Promise<string | null> {
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

export async function fetchCoverArtAssetUrl(payload: CoverArtResourceRequest): Promise<string | null> {
  const key = coverArtCacheKey(payload);
  const cached = coverArtCache.get(key);
  if (cached !== undefined) {
    return cached;
  }

  const promise = fetchCoverArtAssetUrlUncached(payload);
  coverArtCache.set(key, promise);

  promise.then((result) => {
    if (result === null) {
      coverArtCache.delete(key);
    }
  });

  return promise;
}
