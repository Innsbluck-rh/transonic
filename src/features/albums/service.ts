import { commands, type AlbumListRequest, type AlbumListResponse, type CoverArtRequest } from '~/bindings';

const coverArtCache = new Map<string, Promise<string | null>>();

export async function fetchAlbumList(payload: AlbumListRequest): Promise<AlbumListResponse> {
  const result = await commands.getAlbumList(payload);
  if (result.status === 'error') {
    throw new Error(result.error);
  }

  return result.data;
}

function coverArtCacheKey(payload: CoverArtRequest) {
  return `${payload.coverArtId}:${payload.size ?? 'full'}`;
}

export function fetchCoverArtDataUrl(payload: CoverArtRequest) {
  const cacheKey = coverArtCacheKey(payload);
  const cached = coverArtCache.get(cacheKey);
  if (cached) {
    return cached;
  }

  const pending = commands
    .getCoverArt(payload)
    .then((response) => {
      if (response.status === 'error') {
        throw new Error(response.error);
      }

      return response.data.dataUrl;
    })
    .catch((invokeError) => {
      console.error(invokeError);
      coverArtCache.delete(cacheKey);
      return null;
    });

  coverArtCache.set(cacheKey, pending);
  return pending;
}
