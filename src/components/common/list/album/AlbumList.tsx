import { Component, For, Show } from 'solid-js';
import { AlbumItem } from './item/AlbumGridItem';
import AlbumListItem from './item/AlbumListItem';

interface AlbumGridProps {
  albums: AlbumItem[];
  noArtistName?: boolean;
  emptyMessage?: string;
  onItemClick?: (album: AlbumItem, e: MouseEvent) => void;
  onItemDblClick?: (album: AlbumItem, e: MouseEvent) => void;
}

const AlbumList: Component<AlbumGridProps> = (props) => {
  return (
    <Show when={props.albums.length > 0} fallback={<div class='text-sm text-zinc-400'>{props.emptyMessage ?? 'No albums available.'}</div>}>
      <div class='flex w-full flex-col'>
        <For each={props.albums}>
          {(album) => (
            <>
              <AlbumListItem
                album={album}
                noArtistName={props.noArtistName}
                onClick={(e) => props.onItemClick?.(album, e)}
                onDblClick={(e) => props.onItemDblClick?.(album, e)}
              />
              <div class='bg-secondary-border h-px w-full'></div>
            </>
          )}
        </For>
      </div>
    </Show>
  );
};

export default AlbumList;
