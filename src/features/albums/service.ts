import { convertFileSrc } from '@tauri-apps/api/core';
import { commands, type CoverArtRequest } from '~/bindings';

export type CoverArtResourceRequest = CoverArtRequest;

const MAX_CONCURRENT_COVER_ART_FETCHES = 4;

const resolvedCoverArtAssets = new Map<string, string | null>();
const inFlightCoverArtAssets = new Map<string, Promise<string | null>>();
const pendingCoverArtFetches: (() => void)[] = [];
let activeCoverArtFetchCount = 0;

function coverArtCacheKey(payload: CoverArtResourceRequest): string {
  return `${payload.profileId.trim()}\0${payload.coverArtId.trim()}\0${payload.size ?? 'full'}`;
}

function normalizedCoverArtPayload(payload: CoverArtResourceRequest): CoverArtResourceRequest {
  return {
    profileId: payload.profileId.trim(),
    coverArtId: payload.coverArtId.trim(),
    size: payload.size ?? null,
  };
}

function drainCoverArtFetchQueue() {
  while (activeCoverArtFetchCount < MAX_CONCURRENT_COVER_ART_FETCHES) {
    const nextFetch = pendingCoverArtFetches.shift();
    if (!nextFetch) {
      return;
    }

    nextFetch();
  }
}

function enqueueCoverArtFetch(task: () => Promise<string | null>): Promise<string | null> {
  return new Promise((resolve) => {
    const run = () => {
      activeCoverArtFetchCount += 1;
      void task()
        .then(resolve)
        .catch((error) => {
          console.error(error);
          resolve(null);
        })
        .finally(() => {
          activeCoverArtFetchCount -= 1;
          drainCoverArtFetchQueue();
        });
    };

    pendingCoverArtFetches.push(run);
    drainCoverArtFetchQueue();
  });
}

export async function fetchCoverArtAssetUrl(payload: CoverArtResourceRequest): Promise<string | null> {
  const normalizedPayload = normalizedCoverArtPayload(payload);
  const key = coverArtCacheKey(normalizedPayload);
  if (resolvedCoverArtAssets.has(key)) {
    return resolvedCoverArtAssets.get(key) ?? null;
  }

  const inFlight = inFlightCoverArtAssets.get(key);
  if (inFlight) {
    return inFlight;
  }

  const request = enqueueCoverArtFetch(async () => {
    try {
      const response = await commands.getCoverArt(normalizedPayload);
      if (response.status === 'error') {
        throw new Error(response.error);
      }

      return convertFileSrc(response.data.localPath);
    } catch (invokeError) {
      console.error(invokeError);
      return null;
    }
  }).then((assetUrl) => {
    resolvedCoverArtAssets.set(key, assetUrl);
    inFlightCoverArtAssets.delete(key);
    return assetUrl;
  });

  inFlightCoverArtAssets.set(key, request);
  return request;
}
