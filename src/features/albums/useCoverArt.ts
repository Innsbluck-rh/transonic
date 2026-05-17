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

export function useCoverArt(
  coverArtId: Accessor<string | null | undefined>,
  size: number = CoverArtSizes.md,
  options: CoverArtOptions = {}
): CoverArtState {
  const request = createMemo(() => {
    const id = coverArtId();
    if (!id) return null;
    const profileId = sessionStore.activeSession?.profileId;
    if (!profileId) return null;
    return { profileId, coverArtId: id, size, cachedFallbackSizes: options.cachedFallbackSizes };
  });

  const [coverArt] = createResource(request, fetchCoverArtAssetUrl);

  return {
    src: () => coverArt(),
    loading: () => coverArt.loading,
  };
}
