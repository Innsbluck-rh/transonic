import { Component, For, Show } from 'solid-js';
import type { AlbumListItem } from '~/bindings';
import AlbumGridItem from './item/AlbumGridItem';

interface AlbumHorizontalListProps {
  class?: string;
  albums: AlbumListItem[];
  emptyMessage?: string;
  onItemClick?: (album: AlbumListItem, e: MouseEvent) => void;
  onItemDblClick?: (album: AlbumListItem, e: MouseEvent) => void;
  onItemArtClick?: (album: AlbumListItem, e: MouseEvent) => void;
  onItemArtDblClick?: (album: AlbumListItem, e: MouseEvent) => void;
  onItemContextMenu?: (album: AlbumListItem, e: MouseEvent) => void;
}

const AlbumHorizontalList: Component<AlbumHorizontalListProps> = (props) => {
  return (
    <Show when={props.albums.length > 0} fallback={<div class='px-2 py-6 text-sm text-zinc-400'>{props.emptyMessage ?? 'No albums available.'}</div>}>
      <div class={`flex h-auto flex-row gap-3 overflow-x-auto overflow-y-hidden py-2 ${props.class}`}>
        <For each={props.albums}>
          {(album) => (
            <AlbumGridItem
              class='w-28'
              album={album}
              onClick={(e) => props.onItemClick?.(album, e)}
              onDblClick={(e) => props.onItemDblClick?.(album, e)}
              onArtClick={(e) => props.onItemArtClick?.(album, e)}
              onArtDblClick={(e) => props.onItemArtDblClick?.(album, e)}
            />
          )}
        </For>
      </div>
    </Show>
  );
};

export default AlbumHorizontalList;
