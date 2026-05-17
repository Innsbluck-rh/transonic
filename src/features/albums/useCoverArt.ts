import { type Accessor, createMemo, createResource } from 'solid-js';
import { sessionStore } from '~/stores/SessionStore';
import { CoverArtSizes } from './CoverArtSizes';
import { fetchCoverArtAssetUrl } from './service';

export interface CoverArtState {
  /** The resolved asset URL. `undefined` while loading or when no coverArtId is provided; `null` when the fetch failed. */
  src: Accessor<string | null | undefined>;
  /** `true` while the backend request is in flight. */
  loading: Accessor<boolean>;
}

export function useCoverArt(coverArtId: Accessor<string | null | undefined>, size: number = CoverArtSizes.md): CoverArtState {
  const request = createMemo(() => {
    const id = coverArtId();
    if (!id) return null;
    const profileId = sessionStore.activeSession?.profileId;
    if (!profileId) return null;
    return { profileId, coverArtId: id, size };
  });

  const [coverArt] = createResource(request, fetchCoverArtAssetUrl);

  return {
    src: () => coverArt(),
    loading: () => coverArt.loading,
  };
}
