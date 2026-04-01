import { Component } from 'solid-js';
import type { AlbumListItem } from '~/bindings';
import AlbumCover from './AlbumCover';

interface AlbumGridItemProps {
  class?: string;
  album: AlbumListItem;
  clickable?: boolean;
  onClick?: (e: MouseEvent) => void;
  onDblClick?: (e: MouseEvent) => void;
  onArtClick?: (e: MouseEvent) => void;
  onArtDblClick?: (e: MouseEvent) => void;
}

const AlbumGridItem: Component<AlbumGridItemProps> = (props) => {
  const isClickable = props.clickable === undefined ? true : props.clickable;

  return (
    <div
      class={`group flex flex-col shrink-0 gap-0.5 ${props.class}`}
      classList={{
        'cursor-pointer': isClickable,
      }}
      onClick={(e) => props.onArtClick?.(e)}
      onDblClick={(e) => props.onArtDblClick?.(e)}
    >
      <div class='relative w-fit h-fit' onClick={(e) => props.onClick?.(e)} onDblClick={(e) => props.onDblClick?.(e)}>
        <AlbumCover albumName={props.album.name} coverArtId={props.album.coverArtId} year={props.album.year} />
        <div
          class='absolute top-0 bottom-0 left-0 right-0  bg-[#00000020] hidden'
          classList={{
            'group-hover:flex': isClickable,
          }}
        >
          {/* <Icon class='m-auto text-white text-2xl' icon='material-symbols:play-arrow' /> */}
        </div>
      </div>
      <div class='flex flex-col gap-0'>
        <p class='truncate text-sm' title={props.album.name}>
          {props.album.name}
        </p>
        <p class='truncate text-xs text-zinc-500' title={props.album.artist ?? undefined}>
          {props.album.artist ?? 'unknown artist'}
        </p>
      </div>
    </div>
  );
};

export default AlbumGridItem;
