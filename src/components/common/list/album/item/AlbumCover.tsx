import { Component, Match, Switch } from 'solid-js';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArt } from '~/features/albums/useCoverArt';
import PseudoAlbumCover from './PseudoAlbumCover';

interface AlbumCoverProps {
  albumName: string;
  coverArtId?: string | null;
  year?: number | null;
}

const AlbumCover: Component<AlbumCoverProps> = (props) => {
  const { src, loading } = useCoverArt(() => props.coverArtId ?? null, CoverArtSizes.lg);

  return (
    <div class='border-secondary-border aspect-square border object-cover'>
      <Switch>
        <Match when={src()}>
          {(assetUrl) => (
            <img class='h-full w-full object-cover' src={assetUrl()} alt={`${props.albumName} cover art`} loading='lazy' decoding='async' />
          )}
        </Match>
        <Match when={loading()}>
          <div class='bg-primary-inner-surface h-full w-full' />
        </Match>
        <Match when={true}>
          <PseudoAlbumCover coverArtId={props.coverArtId} year={props.year} />
        </Match>
      </Switch>
    </div>
  );
};

export default AlbumCover;
