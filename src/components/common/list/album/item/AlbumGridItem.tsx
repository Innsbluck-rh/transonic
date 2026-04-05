import { Component } from 'solid-js';
import type { AlbumListItem, ArtistAlbum, FolderStructureAlbumItem } from '~/bindings';
import AlbumCover from './AlbumCover';

export type AlbumItem = AlbumListItem | ArtistAlbum | FolderStructureAlbumItem;

interface AlbumGridItemProps {
  class?: string;
  album: AlbumItem;
  onClick?: (e: MouseEvent) => void;
  onDblClick?: (e: MouseEvent) => void;
  onContextMenu?: (e: MouseEvent) => void;
  onArtClick?: (e: MouseEvent) => void;
  onArtDblClick?: (e: MouseEvent) => void;
}

const AlbumGridItem: Component<AlbumGridItemProps> = (props) => {
  const isClickable = !!(props.onClick || props.onDblClick || props.onArtClick || props.onArtDblClick);

  return (
    <div
      class={`group flex shrink-0 flex-col ${props.class}`}
      onClick={(e) => props.onArtClick?.(e)}
      onDblClick={(e) => props.onArtDblClick?.(e)}
      onContextMenu={(e) => props.onContextMenu?.(e)}
      classList={{
        'cursor-pointer': isClickable,
      }}
    >
      <div
        class='ripple relative'
        classList={{
          'cursor-pointer': isClickable,
        }}
        onClick={(e) => props.onClick?.(e)}
        onDblClick={(e) => props.onDblClick?.(e)}
      >
        <AlbumCover albumName={props.album.name ?? '[unknown]'} coverArtId={props.album.coverArtId} year={props.album.year} />
        {/* <div
          class='absolute top-0 right-0 bottom-0 left-0 hidden bg-[#00000000]'
          classList={{
            'group-hover:flex': isClickable,
          }}
        >
        </div> */}
      </div>
      <div class='mt-1 flex flex-col gap-0'>
        <p class='text-md truncate font-bold' title={props.album.name ?? '[unknown]'}>
          {props.album.name}
        </p>
        <p class='text-secondary-text truncate text-xs' title={props.album.artist ?? undefined}>
          {props.album.artist ?? 'unknown artist'}
        </p>
      </div>
    </div>
  );
};

export default AlbumGridItem;
