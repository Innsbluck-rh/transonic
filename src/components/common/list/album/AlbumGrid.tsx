import { Component, For, Show } from 'solid-js';
import AlbumGridItem, { AlbumItem } from './item/AlbumGridItem';

interface AlbumGridProps {
  albums: AlbumItem[];
  noArtistName?: boolean;
  inline?: boolean;
  emptyMessage?: string;
  onItemClick?: (album: AlbumItem, e: MouseEvent) => void;
  onItemDblClick?: (album: AlbumItem, e: MouseEvent) => void;
  onItemArtClick?: (album: AlbumItem, e: MouseEvent) => void;
  onItemArtDblClick?: (album: AlbumItem, e: MouseEvent) => void;
}

export const ALBUM_GRID_MIN_CELL_PX = 112;
export const ALBUM_GRID_COLUMN_GAP_PX = 10;

/** Number of columns the grid lays out at the given content width (same rule as `auto-fill`). */
export function getAlbumGridColumnCount(width: number): number {
  return Math.max(1, Math.floor((width + ALBUM_GRID_COLUMN_GAP_PX) / (ALBUM_GRID_MIN_CELL_PX + ALBUM_GRID_COLUMN_GAP_PX)));
}

const AlbumGrid: Component<AlbumGridProps> = (props) => {
  return (
    <Show when={props.albums.length > 0} fallback={<div class='px-2 py-6 text-sm text-zinc-400'>{props.emptyMessage ?? 'No albums available.'}</div>}>
      <div
        class='grid w-full gap-y-3'
        style={{
          'grid-template-columns': `repeat(auto-fill, minmax(${ALBUM_GRID_MIN_CELL_PX}px, 1fr))`,
          'column-gap': `${ALBUM_GRID_COLUMN_GAP_PX}px`,
        }}
      >
        <For each={props.albums}>
          {(album) => (
            <AlbumGridItem
              album={album}
              inline={props.inline}
              noArtistName={props.noArtistName}
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
