import { Component, createMemo, createResource, Show } from 'solid-js';
import { fetchCoverArtAssetUrl } from '~/features/albums/service';
import { sessionStore } from '~/stores/SessionStore';
import PseudoAlbumCover from './PseudoAlbumCover';

const DEFAULT_COVER_ART_SIZE = 224;

interface AlbumCoverProps {
  albumName: string;
  coverArtId?: string | null;
  year?: number | null;
}

const AlbumCover: Component<AlbumCoverProps> = (props) => {
  const request = createMemo(() => {
    if (!props.coverArtId) {
      return null;
    }
    const profileId = sessionStore.activeSession?.profileId;
    if (!profileId) {
      return null;
    }

    return {
      profileId,
      coverArtId: props.coverArtId,
      size: DEFAULT_COVER_ART_SIZE,
    };
  });
  const [coverArt] = createResource(request, (payload) => fetchCoverArtAssetUrl(payload));

  return (
    <div class='border-secondary-border aspect-square border object-cover'>
      <Show when={coverArt()} fallback={<PseudoAlbumCover coverArtId={props.coverArtId} year={props.year} />}>
        {(assetUrl) => (
          <img class='h-full w-full object-cover' src={assetUrl()} alt={`${props.albumName} cover art`} loading='lazy' decoding='async' />
        )}
      </Show>
    </div>
  );
};

export default AlbumCover;
