import { Component, For, Show } from 'solid-js';
import AlbumGridItem, { AlbumItem } from './item/AlbumGridItem';

interface AlbumGridProps {
  albums: AlbumItem[];
  emptyMessage?: string;
  onItemClick?: (album: AlbumItem, e: MouseEvent) => void;
  onItemDblClick?: (album: AlbumItem, e: MouseEvent) => void;
  onItemArtClick?: (album: AlbumItem, e: MouseEvent) => void;
  onItemArtDblClick?: (album: AlbumItem, e: MouseEvent) => void;
}

const AlbumGrid: Component<AlbumGridProps> = (props) => {
  return (
    <Show when={props.albums.length > 0} fallback={<div class='px-2 py-6 text-sm text-zinc-400'>{props.emptyMessage ?? 'No albums available.'}</div>}>
      <div class='grid w-full grid-cols-[repeat(auto-fill,minmax(112px,1fr))] gap-x-3 gap-y-3 lg:grid-cols-[repeat(auto-fill,minmax(144px,1fr))]'>
        <For each={props.albums}>
          {(album) => (
            <AlbumGridItem
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

export default AlbumGrid;
