import { Component } from 'solid-js';
import { AlbumListItem } from '~/models/album';
import AlbumCover from './AlbumCover';

interface AlbumGridItemProps {
  album: AlbumListItem;
}

const AlbumGridItem: Component<AlbumGridItemProps> = (props) => {
  return (
    <div class='flex w-28 shrink-0 flex-col gap-1'>
      <AlbumCover albumName={props.album.name} coverArtId={props.album.coverArtId} year={props.album.year} />
      <div class='flex flex-col gap-0 px-1'>
        <p class='truncate text-sm text-zinc-800' title={props.album.name}>
          {props.album.name}
        </p>
        <p class='truncate text-xs leading-none text-zinc-500' title={props.album.artist ?? undefined}>
          {props.album.artist ?? 'unknown artist'}
        </p>
      </div>
    </div>
  );
};

export default AlbumGridItem;
