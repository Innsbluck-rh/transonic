import { Component } from 'solid-js';
import type { AlbumListItem } from '~/bindings';
import AlbumCover from './AlbumCover';

interface AlbumGridItemProps {
  album: AlbumListItem;
  onClick?: (e: MouseEvent) => void;
}

const AlbumGridItem: Component<AlbumGridItemProps> = (props) => {
  return (
    <div
      class='flex w-28 shrink-0 flex-col gap-0.5'
      classList={{
        'cursor-pointer': !!props.onClick,
      }}
      onClick={(e) => props.onClick?.(e)}
    >
      <AlbumCover albumName={props.album.name} coverArtId={props.album.coverArtId} year={props.album.year} />
      <div class='flex flex-col gap-0'>
        <p class='truncate text-sm' title={props.album.name}>
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
