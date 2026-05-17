import { createEffect, createMemo, createSignal, type Accessor } from 'solid-js';
import { sessionStore } from '~/stores/SessionStore';
import { fetchCoverArtAssetUrl, type CoverArtResourceRequest } from './service';

export type CoverArtAssetMap = Record<string, string | null | undefined>;

export interface CoverArtMapState {
  /** Resolved asset URLs keyed by coverArtId. Missing keys are still unresolved or were not requested. */
  assets: Accessor<CoverArtAssetMap>;
  /** Returns the resolved asset URL for a coverArtId. `undefined` means unresolved or no id; `null` means the fetch failed. */
  src: (coverArtId: string | null | undefined) => string | null | undefined;
  /** Returns `true` while a coverArtId is currently being fetched. */
  loading: (coverArtId: string | null | undefined) => boolean;
}

function normalizeCoverArtIds(ids: readonly (string | null | undefined)[]): string[] {
  const uniqueIds = new Set<string>();
  for (const id of ids) {
    const normalizedId = id?.trim();
    if (normalizedId) {
      uniqueIds.add(normalizedId);
    }
  }

  return [...uniqueIds];
}

export interface CoverArtMapOptions {
  cachedFallbackSizes?: CoverArtResourceRequest['cachedFallbackSizes'];
}

function cachedFallbackKey(cachedFallbackSizes: CoverArtResourceRequest['cachedFallbackSizes']): string {
  return [...(cachedFallbackSizes ?? [])].join(',');
}

function coverArtRequestKey(
  profileId: string,
  coverArtId: string,
  size: number,
  cachedFallbackSizes: CoverArtResourceRequest['cachedFallbackSizes']
): string {
  return `${profileId}\0${coverArtId}\0${size}\0${cachedFallbackKey(cachedFallbackSizes)}`;
}

export function useCoverArtMap(
  coverArtIds: Accessor<readonly (string | null | undefined)[]>,
  size: number,
  options: CoverArtMapOptions = {}
): CoverArtMapState {
  const [assets, setAssets] = createSignal<CoverArtAssetMap>({});
  const [loadingIds, setLoadingIds] = createSignal<Set<string>>(new Set());
  const resolvedAssets = new Map<string, string | null>();
  const requestedKeys = new Set<string>();
  const inFlightKeys = new Set<string>();
  let activeScopeKey: string | null = null;
  let activeIds = new Set<string>();

  const request = createMemo(() => {
    const profileId = sessionStore.activeSession?.profileId;
    if (!profileId) {
      return null;
    }

    return {
      profileId,
      scopeKey: `${profileId}\0${size}\0${cachedFallbackKey(options.cachedFallbackSizes)}`,
      coverArtIds: normalizeCoverArtIds(coverArtIds()),
    };
  });

  createEffect(() => {
    const nextRequest = request();
    if (!nextRequest) {
      activeScopeKey = null;
      activeIds = new Set<string>();
      resolvedAssets.clear();
      requestedKeys.clear();
      inFlightKeys.clear();
      setAssets({});
      setLoadingIds(new Set<string>());
      return;
    }

    const { profileId, scopeKey, coverArtIds: ids } = nextRequest;
    if (activeScopeKey !== scopeKey) {
      activeScopeKey = scopeKey;
      activeIds = new Set<string>();
      resolvedAssets.clear();
      requestedKeys.clear();
      inFlightKeys.clear();
      setAssets({});
      setLoadingIds(new Set<string>());
    }

    activeIds = new Set(ids);
    setAssets((prev) => {
      const nextAssets: CoverArtAssetMap = {};
      for (const id of ids) {
        const key = coverArtRequestKey(profileId, id, size, options.cachedFallbackSizes);
        nextAssets[id] = resolvedAssets.has(key) ? resolvedAssets.get(key) : prev[id];
      }
      return nextAssets;
    });
    setLoadingIds((prev) => new Set([...prev].filter((id) => activeIds.has(id))));

    for (const id of ids) {
      const key = coverArtRequestKey(profileId, id, size, options.cachedFallbackSizes);
      if (resolvedAssets.has(key)) {
        continue;
      }
      if (requestedKeys.has(key) || inFlightKeys.has(key)) {
        continue;
      }

      inFlightKeys.add(key);
      setLoadingIds((prev) => new Set(prev).add(id));
      fetchCoverArtAssetUrl({ profileId, coverArtId: id, size, cachedFallbackSizes: options.cachedFallbackSizes })
        .then((assetUrl) => {
          requestedKeys.add(key);
          resolvedAssets.set(key, assetUrl);
          if (activeScopeKey !== scopeKey || !activeIds.has(id)) {
            return;
          }

          setAssets((prev) => ({ ...prev, [id]: assetUrl }));
        })
        .catch((error) => {
          console.error(error);
          requestedKeys.add(key);
          resolvedAssets.set(key, null);
          if (activeScopeKey !== scopeKey || !activeIds.has(id)) {
            return;
          }

          setAssets((prev) => ({ ...prev, [id]: null }));
        })
        .finally(() => {
          inFlightKeys.delete(key);
          setLoadingIds((prev) => {
            const nextLoadingIds = new Set(prev);
            nextLoadingIds.delete(id);
            return nextLoadingIds;
          });
        });
    }
  });

  return {
    assets,
    src: (coverArtId) => {
      const normalizedId = coverArtId?.trim();
      if (!normalizedId) {
        return undefined;
      }

      return assets()[normalizedId];
    },
    loading: (coverArtId) => {
      const normalizedId = coverArtId?.trim();
      if (!normalizedId) {
        return false;
      }

      return loadingIds().has(normalizedId);
    },
  };
}
