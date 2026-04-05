import { convertFileSrc } from '@tauri-apps/api/core';
import { commands } from '~/bindings';

const cache = new Map<string, Promise<string | null>>();

export async function fetchArtistImageAssetUrl(url: string): Promise<string | null> {
  const cached = cache.get(url);
  if (cached !== undefined) {
    return cached;
  }

  const promise = fetchUncached(url);
  cache.set(url, promise);

  promise.then((result) => {
    if (result === null) {
      cache.delete(url);
    }
  });

  return promise;
}

async function fetchUncached(url: string): Promise<string | null> {
  try {
    const response = await commands.getArtistImage({ url });
    if (response.status === 'error') {
      throw new Error(response.error);
    }

    return convertFileSrc(response.data.localPath);
  } catch (invokeError) {
    console.error(invokeError);
    return null;
  }
}
