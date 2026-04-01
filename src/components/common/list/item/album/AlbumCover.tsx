import { Component, createMemo, createResource, Show } from 'solid-js';
import { fetchCoverArtDataUrl } from '~/features/albums/service';
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

    return {
      coverArtId: props.coverArtId,
      size: DEFAULT_COVER_ART_SIZE,
    };
  });
  const [coverArt] = createResource(request, (payload) => fetchCoverArtDataUrl(payload));

  return (
    <Show when={coverArt()} fallback={<PseudoAlbumCover coverArtId={props.coverArtId} year={props.year} />}>
      {(dataUrl) => (
        <img
          class='h-28 w-28 border border-zinc-200 object-cover'
          src={dataUrl()}
          alt={`${props.albumName} cover art`}
          loading='lazy'
          decoding='async'
        />
      )}
    </Show>
  );
};

export default AlbumCover;
