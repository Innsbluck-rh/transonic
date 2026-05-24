import { createMemo, createResource, type Accessor } from 'solid-js';
import { sessionStore } from '~/stores/SessionStore';
import { CoverArtSizes } from './CoverArtSizes';
import { fetchCoverArtAssetUrl, type CoverArtResourceRequest } from './service';

export interface CoverArtState {
  /** The resolved asset URL. `undefined` while loading or when no coverArtId is provided; `null` when the fetch failed. */
  src: Accessor<string | null | undefined>;
  /** `true` while the backend request is in flight. */
  loading: Accessor<boolean>;
}

export interface CoverArtOptions {
  cachedFallbackSizes?: CoverArtResourceRequest['cachedFallbackSizes'];
}

type CoverArtResourceValue = {
  requestKey: string;
  src: string | null;
};

function normalizeCachedFallbackSizes(sizes: CoverArtResourceRequest['cachedFallbackSizes']): number[] {
  const normalizedSizes: number[] = [];
  for (const size of sizes ?? []) {
    if (!Number.isInteger(size) || size <= 0 || normalizedSizes.includes(size)) {
      continue;
    }
    normalizedSizes.push(size);
  }
  return normalizedSizes;
}

function coverArtRequestKey(payload: CoverArtResourceRequest): string {
  return [
    payload.profileId.trim(),
    payload.coverArtId.trim(),
    payload.size ?? 'full',
    normalizeCachedFallbackSizes(payload.cachedFallbackSizes).join(','),
  ].join('\0');
}

export function useCoverArt(
  coverArtId: Accessor<string | null | undefined>,
  size: number = CoverArtSizes.md,
  options: CoverArtOptions = {}
): CoverArtState {
  const request = createMemo(() => {
    const id = coverArtId()?.trim();
    if (!id) return null;
    const profileId = sessionStore.activeSession?.profileId.trim();
    if (!profileId) return null;
    return { profileId, coverArtId: id, size, cachedFallbackSizes: options.cachedFallbackSizes };
  });
  const requestKey = createMemo(() => {
    const nextRequest = request();
    return nextRequest ? coverArtRequestKey(nextRequest) : null;
  });

  const [coverArt] = createResource(requestKey, async (key): Promise<CoverArtResourceValue | undefined> => {
    const nextRequest = request();
    if (!nextRequest || coverArtRequestKey(nextRequest) !== key) {
      return undefined;
    }

    return {
      requestKey: key,
      src: await fetchCoverArtAssetUrl(nextRequest),
    };
  });

  return {
    src: () => {
      const key = requestKey();
      if (!key) {
        return undefined;
      }

      const nextCoverArt = coverArt();
      if (!nextCoverArt || nextCoverArt.requestKey !== key) {
        return undefined;
      }

      return nextCoverArt.src;
    },
    loading: () => requestKey() !== null && coverArt.loading,
  };
}
