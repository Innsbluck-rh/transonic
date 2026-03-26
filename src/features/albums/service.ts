import { invoke } from '@tauri-apps/api/core';
import { AlbumListRequest, AlbumListResponse, CoverArtRequest, CoverArtResponse } from '~/models/album';

const coverArtCache = new Map<string, Promise<string | null>>();

export async function fetchAlbumList(payload: AlbumListRequest) {
  return invoke<AlbumListResponse>('get_album_list', {
    payload,
  });
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

  const pending = invoke<CoverArtResponse>('get_cover_art', {
    payload,
  })
    .then((response) => response.dataUrl)
    .catch((invokeError) => {
      console.error(invokeError);
      coverArtCache.delete(cacheKey);
      return null;
    });

  coverArtCache.set(cacheKey, pending);
  return pending;
}
