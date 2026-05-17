import { convertFileSrc } from '@tauri-apps/api/core';
import { commands, type CoverArtRequest } from '~/bindings';

export type CoverArtResourceRequest = Omit<CoverArtRequest, 'cachedFallbackSizes'> & {
  cachedFallbackSizes?: readonly number[] | null;
};

const MAX_CONCURRENT_COVER_ART_FETCHES = 4;

const resolvedCoverArtAssets = new Map<string, string | null>();
const inFlightCoverArtAssets = new Map<string, Promise<string | null>>();
const pendingCoverArtFetches: (() => void)[] = [];
let activeCoverArtFetchCount = 0;

function normalizeCachedFallbackSizes(sizes: readonly number[] | null | undefined): number[] {
  const normalizedSizes: number[] = [];
  for (const size of sizes ?? []) {
    if (!Number.isInteger(size) || size <= 0 || normalizedSizes.includes(size)) {
      continue;
    }
    normalizedSizes.push(size);
  }
  return normalizedSizes;
}

function coverArtCacheKey(payload: CoverArtRequest): string {
  return [payload.profileId.trim(), payload.coverArtId.trim(), payload.size ?? 'full', payload.cachedFallbackSizes?.join(',') ?? ''].join('\0');
}

function normalizedCoverArtPayload(payload: CoverArtResourceRequest): CoverArtRequest {
  return {
    profileId: payload.profileId.trim(),
    coverArtId: payload.coverArtId.trim(),
    size: payload.size ?? null,
    cachedFallbackSizes: normalizeCachedFallbackSizes(payload.cachedFallbackSizes),
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

async function fetchCachedCoverArtAssetUrl(payload: CoverArtRequest): Promise<string | null | undefined> {
  try {
    const response = await commands.getCoverArtCached(payload);
    if (response.status === 'error') {
      console.warn(response.error);
      return undefined;
    }
    return response.data ? convertFileSrc(response.data.localPath) : undefined;
  } catch (invokeError) {
    console.warn(invokeError);
    return undefined;
  }
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

  const request = (async () => {
    const cachedAssetUrl = await fetchCachedCoverArtAssetUrl(normalizedPayload);
    if (cachedAssetUrl !== undefined) {
      return cachedAssetUrl;
    }

    return enqueueCoverArtFetch(async () => {
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
    });
  })().then((assetUrl) => {
    resolvedCoverArtAssets.set(key, assetUrl);
    inFlightCoverArtAssets.delete(key);
    return assetUrl;
  });

  inFlightCoverArtAssets.set(key, request);
  return request;
}
